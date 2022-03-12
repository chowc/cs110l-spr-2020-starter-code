use libc::{exit, stat};
use nix::Error;
use nix::unistd::ForkResult::Child;
use crate::debugger_command::DebuggerCommand;
use crate::inferior::{Inferior, Status};
use rustyline::error::ReadlineError;
use rustyline::Editor;

pub struct Debugger {
    target: String,
    history_path: String,
    readline: Editor<()>,
    inferior: Option<Inferior>,
}

impl Debugger {
    /// Initializes the debugger.
    pub fn new(target: &str) -> Debugger {
        // TODO (milestone 3): initialize the DwarfData

        let history_path = format!("{}/.deet_history", std::env::var("HOME").unwrap());
        let mut readline = Editor::<()>::new();
        // Attempt to load history from ~/.deet_history if it exists
        let _ = readline.load_history(&history_path);

        Debugger {
            target: target.to_string(),
            history_path,
            readline,
            inferior: None,
        }
    }

    pub fn run(&mut self) {
        loop {
            match self.get_next_command() {
                DebuggerCommand::Run(args) => {
                    if self.inferior.is_some() {
                        let _ = self.inferior.take().unwrap().kill();
                    }
                    if let Some(mut inferior) = Inferior::new(&self.target, &args) {
                        // Create the inferior
                        let _ = inferior.cont();
                        let result = inferior.wait(None);
                        print_status(result);
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
                    print_status(result);
                }
                DebuggerCommand::Quit => {
                    if self.inferior.is_some() {
                        let inferior = self.inferior.as_mut().unwrap();
                        let _ = inferior.kill();
                        let result = inferior.wait(None);
                        print_status(result);
                    }
                    return;
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
}

fn print_status(result: Result<Status, nix::Error>) {
    match result {
        Ok(Status::Exited(exit_code)) => {
            println!("Child exited (status {})", exit_code);
        }
        Ok(Status::Signaled(signal)) => {
            println!("Child stopped (signal {})", signal);
        }
        Ok(Status::Stopped(signal, _)) => {
            println!("Child stopped (signal {})", signal);
        }
        _ => {}
    }
}