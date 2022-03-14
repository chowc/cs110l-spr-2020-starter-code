use std::io::Error;
use std::mem::size_of;
use std::os::unix::process::CommandExt;
use nix::sys::ptrace;
use nix::sys::signal;
use nix::sys::wait::{waitpid, WaitPidFlag, WaitStatus};
use nix::unistd::Pid;
use std::process::{Child, Command};
use gimli::SectionId::DebugInfo;
use libc::{wait};
use nix::sys::ptrace::traceme;
use nix::sys::signal::Signal;
use crate::dwarf_data;
use crate::dwarf_data::DwarfData;

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
    pub fn new(target: &str, args: &Vec<String>, breakpoints: &Vec<u64>) -> Option<Inferior> {
        unsafe {
            let child = Command::new(target)
                .args(args)
                .pre_exec(child_traceme)
                .spawn()
                .ok()?;
            let mut i = Inferior { child };
            // When a process that has PTRACE_TRACEME enabled calls exec, the OS will load the specified program into the process,
            // and then, before the program starts running, it will pause the process with SIGTRAP.
            let status = i.wait(None).ok()?;
            let signal = match status {
                Status::Stopped(signal, _) => {
                    Some(signal)
                }
                _ => None
            }?;
            for addr in breakpoints {
                i.write_byte(*addr, 0xcc).unwrap();
            }
            // wait until child process turns its status to Stopped
            match signal {
                Signal::SIGTRAP => {
                    Some(i)
                }
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

    // Normally, SIGINT (triggered by Ctrl-C) will terminate a process, but if a process is being traced under ptrace,
    // SIGINT will cause it to temporarily stop instead, as if it were sent SIGSTOP.
    /// Calls cont on this inferior to get the stopped child process start executing again.
    pub fn cont(&self) -> Result<(), Error> {
        ptrace::cont(self.pid(), None).or(Err(std::io::Error::new(
            std::io::ErrorKind::Other,
            "ptrace cont failed",
        )))
    }

    /// Calls kill on this inferior to kill it and reap the process.
    pub fn kill(&mut self) -> std::io::Result<()> {
        self.child.kill()
    }

    /// print_backtrace
    pub fn print_backtrace(&self, dwarf_data: &DwarfData) -> Result<(), nix::Error> {
        let regs = ptrace::getregs(self.pid())?;
        let rip = regs.rip as usize;
        println!("%rip register: {:#x}", rip);
        // let rsp = regs.rsp as usize;
        let line = dwarf_data.get_line_from_addr(rip).unwrap();
        let func = dwarf_data.get_function_from_addr(rip).unwrap();
        println!("#{} (#{})", func, line);
        Ok(())
    }

    pub(crate) fn write_byte(&mut self, addr: u64, val: u8) -> Result<u8, nix::Error> {
        let aligned_addr = align_addr_to_word(addr);
        let byte_offset = addr - aligned_addr;
        let word = ptrace::read(self.pid(), aligned_addr as ptrace::AddressType)? as u64;
        let orig_byte = (word >> 8 * byte_offset) & 0xff;
        let masked_word = word & !(0xff << 8 * byte_offset);
        let updated_word = masked_word | ((val as u64) << 8 * byte_offset);
        ptrace::write(
            self.pid(),
            aligned_addr as ptrace::AddressType,
            updated_word as *mut std::ffi::c_void,
        )?;
        Ok(orig_byte as u8)
    }
}

fn align_addr_to_word(addr: u64) -> u64 {
    addr & (-(size_of::<u64>() as i64) as u64)
}