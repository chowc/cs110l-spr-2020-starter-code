mod request;
mod response;

use std::borrow::BorrowMut;
use std::collections::HashMap;
use std::sync::Arc;
use clap::Clap;
use rand::{Rng, SeedableRng};
use tokio::net::{TcpListener, TcpStream};
use async_std::channel::{unbounded};
use std::thread;
use std::time::Duration;
use http::Request;
use log::log;
use tokio::sync::{mpsc, Mutex};
use tokio::sync::mpsc::unbounded_channel;
use tokio::task;
use tokio::time::delay_for;

/// Contains information parsed from the command-line invocation of balancebeam. The Clap macros
/// provide a fancy way to automatically construct a command-line argument parser.
#[derive(Clap, Debug)]
#[clap(about = "Fun with load balancing")]
struct CmdOptions {
    #[clap(
        short,
        long,
        about = "IP/port to bind to",
        default_value = "0.0.0.0:1100"
    )]
    bind: String,
    #[clap(short, long, about = "Upstream host to forward requests to")]
    upstream: Vec<String>,
    #[clap(
        long,
        about = "Perform active health checks on this interval (in seconds)",
        default_value = "10"
    )]
    active_health_check_interval: usize,
    #[clap(
    long,
    about = "Path to send request to for active health checks",
    default_value = "/"
    )]
    active_health_check_path: String,
    #[clap(
        long,
        about = "Maximum number of requests to accept per IP per minute (0 = unlimited)",
        default_value = "0"
    )]
    max_requests_per_minute: usize,
}

/// Contains information about the state of balancebeam (e.g. what servers we are currently proxying
/// to, what servers have failed, rate limiting counts, etc.)
///
/// You should add fields to this struct in later milestones.
struct ProxyState {
    /// How frequently we check whether upstream servers are alive (Milestone 4)
    #[allow(dead_code)]
    active_health_check_interval: usize,
    /// Where we should send requests when doing active health checks (Milestone 4)
    #[allow(dead_code)]
    active_health_check_path: String,
    /// Maximum number of requests an individual IP can make in a minute (Milestone 5)
    #[allow(dead_code)]
    max_requests_per_minute: usize,
    /// Lists of servers that we are proxying to
    upstream_addresses: Vec<String>,
    /// Request traffic record
    traffic_record: HashMap<String, u64>,
}

/// Represent a upstream server and its health state.
#[derive(Debug)]
struct UpStream {
    address: String,
    state: UpstreamState,
}

#[derive(Debug)]
enum UpstreamState {
    Health,
    Ill,
}

#[tokio::main]
async fn main() {
    // Initialize the logging library. You can print log messages using the `log` macros:
    // https://docs.rs/log/0.4.8/log/ You are welcome to continue using print! statements; this
    // just looks a little prettier.
    if let Err(_) = std::env::var("RUST_LOG") {
        std::env::set_var("RUST_LOG", "debug");
    }
    pretty_env_logger::init();

    // Parse the command line arguments passed to this program
    let options = CmdOptions::parse();
    if options.upstream.len() < 1 {
        log::error!("At least one upstream server must be specified using the --upstream option.");
        std::process::exit(1);
    }

    // Start listening for connections
    let mut listener = match TcpListener::bind(&options.bind).await {
        Ok(listener) => listener,
        Err(err) => {
            log::error!("Could not bind to {}: {}", options.bind, err);
            std::process::exit(1);
        }
    };
    log::info!("Listening for requests on {}", options.bind);

    let proxy_state = ProxyState {
        upstream_addresses: options.upstream,
        active_health_check_interval: options.active_health_check_interval,
        active_health_check_path: options.active_health_check_path,
        max_requests_per_minute: options.max_requests_per_minute,
        traffic_record: HashMap::new(),
    };
    let (sender, mut receiver) = unbounded();
    let mut sender = sender.clone();

    let upstream_addresses = proxy_state.upstream_addresses.clone();
    let active_health_check_path = proxy_state.active_health_check_path.clone();
    let active_health_check_interval = proxy_state.active_health_check_interval;
    let state = Arc::new(Mutex::new(proxy_state));

    let handler = task::spawn(async move {
        loop {
            for address in &upstream_addresses {
                let path = format!("{}{}{}", "http://", address, active_health_check_path);
                log::info!("health check address {}", &path);
                let mut conn = match TcpStream::connect(address).await {
                    Err(err) => {
                        log::error!("Failed to connect to upstream {}: {}, remove from health servers", address, err);
                        sender.send(UpStream { address: address.clone(), state: UpstreamState::Ill }).await;
                        continue;
                    },
                    Ok(other) => {
                        other
                    }
                };
                let request = Request::get(&path).body(vec![]).unwrap();
                if let Err(error) = request::write_to_stream(&request, &mut conn).await {
                    log::error!("Failed to send request to upstream {}: {}", address, error);
                    sender.send(UpStream { address: address.clone(), state: UpstreamState::Ill }).await;
                    continue;
                }
                let response = match response::read_from_stream(&mut conn, request.method()).await {
                    Ok(response) => response,
                    Err(error) => {
                        log::error!("Error reading response from server: {:?}", error);
                        sender.send(UpStream { address: address.clone(), state: UpstreamState::Ill }).await;
                        continue;
                    }
                };
                let code = response.status().as_u16();
                log::info!("health check return status {}, {}", &path, code);
                if code != 200 {
                    sender.send(UpStream { address: address.clone(), state: UpstreamState::Ill }).await;
                } else {
                    sender.send(UpStream { address: address.clone(), state: UpstreamState::Health }).await;
                }
            }
            delay_for(Duration::from_secs(active_health_check_interval as u64)).await;
        }
    });
    let state_clone = Arc::clone(&state);

    let _ = task::spawn(async move {
        loop {
            delay_for(Duration::from_secs(60)).await;
            let mut state = state_clone.lock().await;
            state.traffic_record = HashMap::new();
        }
    });
    loop {
        let stream = match listener.accept().await {
            Ok((stream, _)) => {
                stream
            }
            Err(e) => {
                log::error!("listener accept got error {}", e);
                continue;
            }
        };

        loop {
            let msg = match receiver.try_recv() {
                Ok(msg) => {
                    msg
                }
                Err(e) => {
                    log::error!("try_recv fail {}", e);
                    break;
                }
            };
            let mut state = state.lock().await;
            log::info!("channel msg {:?}", msg);
            match msg.state {
                UpstreamState::Ill => {
                    state.upstream_addresses.retain(|f| { f != &msg.address });
                    log::error!("after retain upstream_addresses {:?}", state.upstream_addresses);
                }
                UpstreamState::Health => {
                    if state.upstream_addresses.contains(&msg.address) {
                        continue;
                    }
                    state.upstream_addresses.push(msg.address.clone());
                }
            }
        }
        // Handle the connection!
        let state = Arc::clone(&state);
        task::spawn(handle_connection(stream, state));
    }
}

async fn connect_to_upstream(state: &mut ProxyState) -> Result<TcpStream, std::io::Error> {
    log::info!("upstream_addresses {:?}", &state.upstream_addresses);
    let mut rng = rand::rngs::StdRng::from_entropy();
    loop {
        let upstream_idx = rng.gen_range(0, state.upstream_addresses.len());
        let mut upstream_ip = &state.upstream_addresses[upstream_idx];
        match TcpStream::connect(upstream_ip).await {
            Err(err) => {
                log::error!("Failed to connect to upstream {}: {}, remove from health servers", upstream_ip, err);
                let removed_upstream = state.upstream_addresses.remove(upstream_idx);
            }
            other => {
                return other;
            }
        }
    }
}

async fn send_response(client_conn: &mut TcpStream, response: &http::Response<Vec<u8>>) {
    let client_ip = client_conn.peer_addr().unwrap().ip().to_string();
    log::info!("{} <- {}", client_ip, response::format_response_line(&response));
    if let Err(error) = response::write_to_stream(&response, client_conn).await {
        log::warn!("Failed to send response to client: {}", error);
        return;
    }
}

async fn handle_connection(mut client_conn: TcpStream, state: Arc<Mutex<ProxyState>>) {
    let client_ip = client_conn.peer_addr().unwrap().ip().to_string();
    log::info!("Connection received from {}", client_ip);
    let mut state = state.lock().await;
    // Open a connection to a random destination server
    let mut upstream_conn = match connect_to_upstream(state.borrow_mut()).await {
        Ok(stream) => stream,
        Err(_error) => {
            let response = response::make_http_error(http::StatusCode::BAD_GATEWAY);
            send_response(&mut client_conn, &response).await;
            return;
        }
    };
    let upstream_ip = client_conn.peer_addr().unwrap().ip().to_string();

    // The client may now send us one or more requests. Keep trying to read requests until the
    // client hangs up or we get an error.
    loop {
        // Read a request from the client
        let mut request = match request::read_from_stream(&mut client_conn).await {
            Ok(request) => request,
            // Handle case where client closed connection and is no longer sending requests
            Err(request::Error::IncompleteRequest(0)) => {
                log::debug!("Client finished sending requests. Shutting down connection");
                return;
            }
            // Handle I/O error in reading from the client
            Err(request::Error::ConnectionError(io_err)) => {
                log::info!("Error reading request from client stream: {}", io_err);
                return;
            }
            Err(error) => {
                log::debug!("Error parsing request: {:?}", error);
                let response = response::make_http_error(match error {
                    request::Error::IncompleteRequest(_)
                    | request::Error::MalformedRequest(_)
                    | request::Error::InvalidContentLength
                    | request::Error::ContentLengthMismatch => http::StatusCode::BAD_REQUEST,
                    request::Error::RequestBodyTooLarge => http::StatusCode::PAYLOAD_TOO_LARGE,
                    request::Error::ConnectionError(_) => http::StatusCode::SERVICE_UNAVAILABLE,
                });
                send_response(&mut client_conn, &response).await;
                continue;
            }
        };
        log::info!(
            "{} -> {}: {}",
            client_ip,
            upstream_ip,
            request::format_request_line(&request)
        );
        if state.max_requests_per_minute != 0 {
            if *state.traffic_record.entry(client_ip.clone()).and_modify(|n| *n+=1).or_insert(1) > state.max_requests_per_minute as u64 {
                let response = response::make_http_error(http::StatusCode::TOO_MANY_REQUESTS);
                send_response(&mut client_conn, &response).await;
                return;
            }
        }
        // Add X-Forwarded-For header so that the upstream server knows the client's IP address.
        // (We're the ones connecting directly to the upstream server, so without this header, the
        // upstream server will only know our IP, not the client's.)
        request::extend_header_value(&mut request, "x-forwarded-for", &client_ip);

        // Forward the request to the server
        if let Err(error) = request::write_to_stream(&request, &mut upstream_conn).await {
            log::error!("Failed to send request to upstream {}: {}", upstream_ip, error);
            let response = response::make_http_error(http::StatusCode::BAD_GATEWAY);
            send_response(&mut client_conn, &response).await;
            return;
        }
        log::debug!("Forwarded request to server");

        // Read the server's response
        let response = match response::read_from_stream(&mut upstream_conn, request.method()).await {
            Ok(response) => response,
            Err(error) => {
                log::error!("Error reading response from server: {:?}", error);
                let response = response::make_http_error(http::StatusCode::BAD_GATEWAY);
                send_response(&mut client_conn, &response).await;
                return;
            }
        };
        // Forward the response to the client
        send_response(&mut client_conn, &response).await;
        log::debug!("Forwarded response to client");
    }
}
