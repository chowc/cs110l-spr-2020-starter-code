use crossbeam_channel;
use std::{thread, time};
use std::sync::mpsc::{channel, sync_channel};
use std::sync::Arc;

fn parallel_map<T, U, F>(mut input_vec: Vec<T>, num_threads: usize, f: F) -> Vec<U>
where
    F: FnOnce(T) -> U + Send + Copy + 'static + Sync,
    T: Send + 'static,
    U: Send + 'static + Default,
{
    let mut output_vec: Vec<U> = Vec::with_capacity(input_vec.len());
    let mut senders = vec![];
    let mut receivers = vec![];
    for _ in 0..num_threads {
        let (s, r) = channel();
        senders.push(s);
        receivers.push(r);
    }
    for (idx, e) in input_vec.into_iter().enumerate() {
        let sender = senders[idx%num_threads].clone();
        sender.send((idx, e));
    }
    drop(senders);

    let mut handlers = vec![];
    let (result_sender, result_receiver) = channel();
    let f = Arc::new(f);
    for receiver in receivers {
        let f = f.clone();
        let result_sender = result_sender.clone();
        let handler = thread::spawn(move || {
            loop {
                match receiver.recv() {
                    Ok((idx, e)) => {
                        let result = f(e);
                        result_sender.send((idx, result));
                    },
                    _ => break,
                }
            }
        });
        handlers.push(handler);
    }
    for handler in handlers {
        handler.join();
    }
    drop(result_sender);
    for _ in 0..output_vec.capacity() {
        output_vec.push(U::default());
    }
    for _ in 0..output_vec.capacity() {
        match result_receiver.recv() {
            Ok((idx, r)) => {
                output_vec[idx] = r;
            },
            _ => break,
        }
    }
    output_vec
}

fn main() {
    let v = vec![6, 7, 8, 9, 10, 1, 2, 3, 4, 5, 12, 18, 11, 5, 20];
    let squares = parallel_map(v, 10, |num| {
        println!("{} squared is {}", num, num * num);
        thread::sleep(time::Duration::from_millis(500));
        num * num
    });
    println!("squares: {:?}", squares);
}
