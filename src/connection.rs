//! ## Example
//!
//! ```no_run
//! use pwntools::connection::Process;
//! use pwntools::connection::Connection;
//!
//! # async fn dox() -> std::io::Result<()> {
//!     let mut conn = Process::new(&"./some_binary")?;
//!     conn.send(&b"x".repeat(32)).await?;
//!     conn.send(&0x1337beef_u64.to_le_bytes()).await?;
//!     conn.interactive().await?;
//!     # Ok(())
//! # }
//! ```

use async_trait::async_trait;
use futures::future;
use std::ffi::OsStr;
use std::io;
use std::process::Stdio;
use tokio::io::{AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;
use tokio::process::{Child, ChildStdin, ChildStdout, Command};

use crate::util::{Payload, P64};

pub struct Process {
    _child: Child,
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

#[async_trait]
pub trait Connection {
    async fn send<D: ?Sized + ToVec + Sync>(&mut self, data: &D) -> io::Result<()>;
    async fn sendline<D: ?Sized + ToVec + Sync>(&mut self, data: &D) -> io::Result<()> {
        self.send(data).await?;
        self.send(b"\n").await?;
        Ok(())
    }
    async fn recvline(&mut self) -> io::Result<Vec<u8>> {
        self.recvuntil(b"\n").await
    }
    async fn recvuntil(&mut self, pattern: &[u8]) -> io::Result<Vec<u8>>;
    async fn interactive(self) -> io::Result<()>;
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
            _child: child,
            stdin,
            stdout_reader,
        })
    }
}

#[async_trait]
impl Connection for Process {
    async fn send<D: ?Sized + ToVec + Sync>(&mut self, data: &D) -> io::Result<()> {
        self.stdin.write_all(&data.to_vec()).await?;
        self.stdin.flush().await?;
        Ok(())
    }

    async fn recvuntil(&mut self, pattern: &[u8]) -> io::Result<Vec<u8>> {
        let mut result = vec![];

        let mut buf = [0];
        while self.stdout_reader.read_exact(&mut buf).await.is_ok() {
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

    async fn interactive(mut self) -> io::Result<()> {
        future::try_join(
            tokio::io::copy(&mut tokio::io::stdin(), &mut self.stdin),
            tokio::io::copy(&mut self.stdout_reader, &mut tokio::io::stdout()),
        )
        .await?;

        Ok(())
    }
}

pub struct Remote {
    stream: TcpStream,
}

impl Remote {
    pub async fn new(addr: &str) -> io::Result<Self> {
        let stream = TcpStream::connect(addr).await?;
        Ok(Self { stream })
    }
}

#[async_trait]
impl Connection for Remote {
    async fn send<D: ?Sized + ToVec + Sync>(&mut self, data: &D) -> io::Result<()> {
        self.stream.write_all(data.to_vec().as_slice()).await?;
        self.stream.flush().await?;
        Ok(())
    }

    async fn recvuntil(&mut self, pattern: &[u8]) -> io::Result<Vec<u8>> {
        let mut buf = vec![];
        loop {
            let mut buf_ = [0];
            self.stream.read_exact(&mut buf_).await?;
            buf.extend_from_slice(&buf_[..]);
            if buf.ends_with(pattern) {
                break;
            }
        }
        Ok(buf)
    }

    async fn interactive(self) -> io::Result<()> {
        let (mut read_half, mut write_half) = tokio::io::split(self.stream);
        future::try_join(
            tokio::io::copy(&mut tokio::io::stdin(), &mut write_half),
            tokio::io::copy(&mut read_half, &mut tokio::io::stdout()),
        )
        .await?;
        Ok(())
    }
}
