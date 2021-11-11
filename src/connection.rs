//! ## Example
//!
//! ```no_run
//! use pwntools::connection::Process;
//! use pwntools::connection::Connection;
//!
//! let mut conn = Process::new(&"./some_binary")?;
//! conn.send(&b"x".repeat(32))?;
//! conn.send(&0x1337beef_u64.to_le_bytes())?;
//! conn.interactive()?;
//! # Ok::<_, std::io::Error>(())
//! ```

use std::ffi::OsStr;
use std::io::{self, BufReader, Read, Write};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};

use crate::util::{Payload, P64};

pub struct Process {
    child: Child,
    stdin: ChildStdin,
    stdout_reader: BufReader<ChildStdout>,
}

pub trait ToVec {
    fn to_vec(&self) -> Vec<u8>;
}

impl ToVec for P64 {
    fn to_vec(&self) -> Vec<u8> {
        self.0.to_le_bytes().to_vec()
    }
}

impl ToVec for Payload {
    fn to_vec(&self) -> Vec<u8> {
        self.as_bytes().to_vec()
    }
}

impl ToVec for Vec<u8> {
    fn to_vec(&self) -> Vec<u8> {
        self.clone()
    }
}

impl<const N: usize> ToVec for [u8; N] {
    fn to_vec(&self) -> Vec<u8> {
        self[..].to_vec()
    }
}

impl ToVec for [u8] {
    fn to_vec(&self) -> Vec<u8> {
        self.to_vec()
    }
}

pub trait Connection {
    fn send<D: ?Sized + ToVec>(&mut self, data: &D) -> io::Result<()>;
    fn sendline<D: ?Sized + ToVec>(&mut self, data: &D) -> io::Result<()> {
        self.send(data)?;
        self.send(b"\n")?;
        Ok(())
    }
    fn recvline(&mut self) -> io::Result<Vec<u8>> {
        self.recvuntil(b"\n")
    }
    fn recvuntil(&mut self, pattern: &[u8]) -> io::Result<Vec<u8>>;
    fn interactive(self) -> io::Result<()>;
}

impl Process {
    pub fn new<S>(program: S) -> io::Result<Self>
    where
        S: AsRef<OsStr>,
    {
        let mut child = Command::new(program)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()?;
        let stdin = child.stdin.take().unwrap();
        let stdout = child.stdout.take().unwrap();
        let stdout_reader = BufReader::new(stdout);
        Ok(Self {
            child,
            stdin,
            stdout_reader,
        })
    }
}

impl Connection for Process {
    fn send<D: ?Sized + ToVec>(&mut self, data: &D) -> io::Result<()> {
        self.stdin.write_all(&data.to_vec())?;
        self.stdin.flush()
    }

    fn recvuntil(&mut self, pattern: &[u8]) -> io::Result<Vec<u8>> {
        let mut result = vec![];

        let mut buf = [0; 1];
        while self.stdout_reader.read_exact(&mut buf).is_ok() {
            result.extend_from_slice(&buf);
            if result.ends_with(pattern) {
                return Ok(result);
            }
        }
        Err(io::Error::new(
            io::ErrorKind::Other,
            "`pattern` not found before reaching to the end.",
        ))
    }

    fn interactive(mut self) -> io::Result<()> {
        let mut stdin = self.stdin;

        std::thread::spawn(move || std::io::copy(&mut std::io::stdin(), &mut stdin).unwrap());
        let mut stdout = self.stdout_reader;

        std::thread::spawn(move || std::io::copy(&mut stdout, &mut std::io::stdout()).unwrap());
        self.child.wait()?;

        Ok(())
    }
}
