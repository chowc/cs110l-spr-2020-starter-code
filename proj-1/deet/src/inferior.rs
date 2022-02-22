use std::os::unix::process::CommandExt;
use nix::sys::ptrace;
use nix::sys::signal;
use nix::sys::wait::{waitpid, WaitPidFlag, WaitStatus};
use nix::unistd::Pid;
use std::process::{Child, Command};
use libc::wait;
use nix::sys::ptrace::traceme;
use nix::sys::signal::Signal;

pub enum Status {
    /// Indicates inferior stopped. Contains the signal that stopped the process, as well as the
    /// current instruction pointer that it is stopped at.
    Stopped(signal::Signal, usize),

    /// Indicates inferior exited normally. Contains the exit status code.
    Exited(i32),

    /// Indicates the inferior exited due to a signal. Contains the signal that killed the
    /// process.
    Signaled(signal::Signal),
}

/// This function calls ptrace with PTRACE_TRACEME to enable debugging on a process. You should use
/// pre_exec with Command to call this in the child process.
fn child_traceme() -> Result<(), std::io::Error> {
    ptrace::traceme().or(Err(std::io::Error::new(
        std::io::ErrorKind::Other,
        "ptrace TRACEME failed",
    )))
}

pub struct Inferior {
    child: Child,
}

impl Inferior {
    /// Attempts to start a new inferior process. Returns Some(Inferior) if successful, or None if
    /// an error is encountered.
    pub fn new(target: &str, args: &Vec<String>) -> Option<Inferior> {
        unsafe {
            let child = Command::new(target)
                .args(args)
                .pre_exec(child_traceme)
                .spawn()
                .ok()?;
            let i = Inferior{ child };
            let status = i.wait(None).ok()?;
            let signal = match status {
                Status::Stopped(signal, _) => {
                    Some(signal)
                },
                _ => None,
            }?;
            match signal {
                Signal::SIGTRAP => {
                    Some(i)
                },
                _ => None
            }
        }
    }

/// Returns the pid of this inferior.
pub fn pid(&self) -> Pid {
    nix::unistd::Pid::from_raw(self.child.id() as i32)
}

/// Calls waitpid on this inferior and returns a Status to indicate the state of the process
/// after the waitpid call.
pub fn wait(&self, options: Option<WaitPidFlag>) -> Result<Status, nix::Error> {
    Ok(match waitpid(self.pid(), options)? {
        WaitStatus::Exited(_pid, exit_code) => Status::Exited(exit_code),
        WaitStatus::Signaled(_pid, signal, _core_dumped) => Status::Signaled(signal),
        WaitStatus::Stopped(_pid, signal) => {
            let regs = ptrace::getregs(self.pid())?;
            Status::Stopped(signal, regs.rip as usize)
        }
        other => panic!("waitpid returned unexpected status: {:?}", other),
    })
}

/// Calls cont on this inferior to get the stopped child process start executing again.
pub fn cont(&self) {
    ptrace::cont(self.pid(), None).or(Err(std::io::Error::new(
        std::io::ErrorKind::Other,
        "ptrace cont failed",
    ))).unwrap();
    let status = self.wait(None).ok();
    match status {
        Some(Status::Exited(exit_code)) => {
            println!("Child exited (status {})", exit_code);
        },
        Some(Status::Signaled(signal)) => {
            println!("Child stopped (signal {})", signal);
        },
        Some(Status::Stopped(signal, _)) => {
            println!("Child stopped (signal {})", signal);
        },
        _ => {}
    }
}

}