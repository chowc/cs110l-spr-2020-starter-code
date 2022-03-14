use libc::{exit, stat};
use nix::Error;
use nix::unistd::ForkResult::Child;
use crate::debugger_command::DebuggerCommand;
use crate::inferior::{Inferior, Status};
use rustyline::error::ReadlineError;
use rustyline::Editor;
use crate::dwarf_data::{DwarfData, Error as DwarfError};

pub struct Debugger {
    target: String,
    history_path: String,
    readline: Editor<()>,
    inferior: Option<Inferior>,
    dwarf_data: DwarfData,
    breakpoints: Vec<u64>,
}

impl Debugger {
    /// Initializes the debugger.
    pub fn new(target: &str) -> Debugger {
        let debug_data = match DwarfData::from_file(target) {
            Ok(val) => val,
            Err(DwarfError::ErrorOpeningFile) => {
                println!("Could not open file {}", target);
                std::process::exit(1);
            }
            Err(DwarfError::DwarfFormatError(err)) => {
                println!("Could not load debugging symbols from {}: {:?}", target, err);
                std::process::exit(1);
            }
        };
        debug_data.print();
        let history_path = format!("{}/.deet_history", std::env::var("HOME").unwrap());
        let mut readline = Editor::<()>::new();
        // Attempt to load history from ~/.deet_history if it exists
        let _ = readline.load_history(&history_path);

        Debugger {
            target: target.to_string(),
            history_path,
            readline,
            inferior: None,
            dwarf_data: debug_data,
            breakpoints: vec![],
        }
    }

    pub fn run(&mut self) {
        loop {
            match self.get_next_command() {
                DebuggerCommand::Run(args) => {
                    if self.inferior.is_some() {
                        let _ = self.inferior.take().unwrap().kill();
                    }
                    if let Some(inferior) = Inferior::new(&self.target, &args, &self.breakpoints) {
                        // Create the inferior
                        let _ = inferior.cont();
                        let result = inferior.wait(None);
                        self.print_status(result);
                        self.inferior = Some(inferior);
                    } else {
                        println!("Error starting subprocess");
                    }
                }
                DebuggerCommand::Continue => {
                    if self.inferior.is_none() {
                        println!("run process first");
                        continue;
                    }
                    let inferior = self.inferior.as_ref().unwrap();
                    let _ = inferior.cont();
                    let result = inferior.wait(None);
                    self.print_status(result);
                }
                DebuggerCommand::Quit => {
                    if self.inferior.is_some() {
                        let inferior = self.inferior.as_mut().unwrap();
                        let _ = inferior.kill();
                        let result = inferior.wait(None);
                        self.print_status(result);
                    }
                    return;
                }
                DebuggerCommand::Backtrace => {
                    match &self.inferior {
                        Some(inferior) => {
                            let _ = inferior.print_backtrace(&self.dwarf_data);
                        }
                        _ => {}
                    }
                }
                DebuggerCommand::BreakPoint(regex) => {
                    let regex = regex.replace("*", "");
                    if let Some(addr) = Debugger::parse_address(regex.as_str()) {
                        println!("Set breakpoint {} at {}",  self.breakpoints.len(), regex);
                        if self.inferior.is_some() {
                            self.inferior.as_mut().unwrap().write_byte(addr, 0xcc).unwrap();
                        }
                        self.breakpoints.push(addr);
                    }
                }
            }
        }
    }

    /// This function prompts the user to enter a command, and continues re-prompting until the user
    /// enters a valid command. It uses DebuggerCommand::from_tokens to do the command parsing.
    ///
    /// You don't need to read, understand, or modify this function.
    fn get_next_command(&mut self) -> DebuggerCommand {
        loop {
            // Print prompt and get next line of user input
            match self.readline.readline("(deet) ") {
                Err(ReadlineError::Interrupted) => {
                    // User pressed ctrl+c. We're going to ignore it
                    println!("Type \"quit\" to exit");
                }
                Err(ReadlineError::Eof) => {
                    // User pressed ctrl+d, which is the equivalent of "quit" for our purposes
                    return DebuggerCommand::Quit;
                }
                Err(err) => {
                    panic!("Unexpected I/O error: {:?}", err);
                }
                Ok(line) => {
                    if line.trim().len() == 0 {
                        continue;
                    }
                    self.readline.add_history_entry(line.as_str());
                    if let Err(err) = self.readline.save_history(&self.history_path) {
                        println!(
                            "Warning: failed to save history file at {}: {}",
                            self.history_path, err
                        );
                    }
                    let tokens: Vec<&str> = line.split_whitespace().collect();
                    if let Some(cmd) = DebuggerCommand::from_tokens(&tokens) {
                        return cmd;
                    } else {
                        println!("Unrecognized command.");
                    }
                }
            }
        }
    }

    fn print_status(&self, result: Result<Status, nix::Error>) {
        match result {
            Ok(Status::Exited(exit_code)) => {
                println!("Child exited (status {})", exit_code);
            }
            Ok(Status::Signaled(signal)) => {
                println!("Child stopped (signal {})", signal);
            }
            Ok(Status::Stopped(signal, rip)) => {
                println!("Child stopped (signal {})", signal);
                // if let Some(func) = self.dwarf_data.get_function_from_addr(rip) {
                //     print!("Stopped at {}", func);
                // }
                if let Some(line) = self.dwarf_data.get_line_from_addr(rip) {
                    println!("{}", line);
                }
            }
            _ => {}
        }
    }

    fn parse_address(addr: &str) -> Option<u64> {
        let addr_without_0x = if addr.to_lowercase().starts_with("0x") {
            &addr[2..]
        } else {
            &addr
        };
        u64::from_str_radix(addr_without_0x, 16).ok()
    }
}