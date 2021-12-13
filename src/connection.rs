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
use tokio::io::{
    AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, BufReader, ReadHalf, WriteHalf,
};
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
pub trait Connection: Sized {
    type Reader: Send + Unpin + AsyncRead;
    type Writer: Send + Unpin + AsyncWrite;

    fn reader_mut(&mut self) -> &mut Self::Reader;
    fn writer_mut(&mut self) -> &mut Self::Writer;
    fn reader_and_writer_mut(&mut self) -> (&mut Self::Reader, &mut Self::Writer);

    async fn send<D: ?Sized + ToVec + Sync>(&mut self, data: &D) -> io::Result<()> {
        let writer = self.writer_mut();
        writer.write_all(data.to_vec().as_slice()).await?;
        writer.flush().await?;
        Ok(())
    }

    async fn recvuntil(&mut self, pattern: &[u8]) -> io::Result<Vec<u8>> {
        let reader = self.reader_mut();
        let mut buf = vec![];
        loop {
            let mut buf_ = [0];
            reader.read_exact(&mut buf_).await?;
            buf.extend_from_slice(&buf_[..]);
            if buf.ends_with(pattern) {
                break;
            }
        }
        Ok(buf)
    }

    async fn interactive(mut self) -> io::Result<()> {
        let (reader, writer) = self.reader_and_writer_mut();
        future::try_join(
            tokio::io::copy(&mut tokio::io::stdin(), writer),
            tokio::io::copy(reader, &mut tokio::io::stdout()),
        )
        .await?;
        Ok(())
    }
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
    type Reader = BufReader<ChildStdout>;
    type Writer = ChildStdin;

    fn reader_mut(&mut self) -> &mut Self::Reader {
        &mut self.stdout_reader
    }

    fn writer_mut(&mut self) -> &mut Self::Writer {
        &mut self.stdin
    }

    fn reader_and_writer_mut(&mut self) -> (&mut Self::Reader, &mut Self::Writer) {
        (&mut self.stdout_reader, &mut self.stdin)
    }
}

pub struct Remote {
    reader: ReadHalf<TcpStream>,
    writer: WriteHalf<TcpStream>,
}

impl Remote {
    pub async fn new(addr: &str) -> io::Result<Self> {
        let (reader, writer) = tokio::io::split(TcpStream::connect(addr).await?);
        Ok(Self { reader, writer })
    }
}

impl Connection for Remote {
    type Reader = ReadHalf<TcpStream>;
    type Writer = WriteHalf<TcpStream>;

    fn reader_mut(&mut self) -> &mut Self::Reader {
        &mut self.reader
    }

    fn writer_mut(&mut self) -> &mut Self::Writer {
        &mut self.writer
    }

    fn reader_and_writer_mut(&mut self) -> (&mut Self::Reader, &mut Self::Writer) {
        (&mut self.reader, &mut self.writer)
    }
}
