use clap::Clap;
use pwntools::pwn::*;
use std::io;

#[derive(Clap)]
struct Opts {
    elf_file: String,
}

fn main() -> io::Result<()> {
    let opts: Opts = Opts::parse();
    let pwn = Pwn::new(&opts.elf_file);
    println!("name: printf, addr: 0x{:x}", pwn.plt("printf").unwrap());

    Ok(())
}
