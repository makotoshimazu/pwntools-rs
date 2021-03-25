//! ## Example
//!
//! ```no_run
//! use pwntools::process::Process;
//!
//! let mut conn = Process::new(&"./some_binary")?;
//! conn.send(&b"x".repeat(32))?;
//! conn.send(&0x1337beef_u64.to_le_bytes())?;
//! conn.interactive()?;
//! # Ok::<_, std::io::Error>(())
//! ```

use std::ffi::OsStr;
use std::io::{self, Write};
use std::process::{Child, ChildStdin, Command, Stdio};

pub struct Process {
    child: Child,
    stdin: ChildStdin,
}

impl Process {
    pub fn new<S>(program: S) -> io::Result<Self>
    where
        S: AsRef<OsStr>,
    {
        let mut child = Command::new(program).stdin(Stdio::piped()).spawn()?;
        let stdin = child.stdin.take().unwrap();
        Ok(Self { child, stdin })
    }

    pub fn send(&mut self, data: &[u8]) -> io::Result<()> {
        self.stdin.write_all(data)
    }

    pub fn sendline(&mut self, data: &[u8]) -> io::Result<()> {
        self.send(data)?;
        self.stdin.write_all(b"\n")
    }

    pub fn interactive(mut self) -> io::Result<()> {
        let mut stdin = self.stdin;
        std::thread::spawn(move || std::io::copy(&mut std::io::stdin(), &mut stdin).unwrap());
        self.child.wait()?;
        Ok(())
    }
}
