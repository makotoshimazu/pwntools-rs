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
    println!("name: printf, addr: 0x{:x}", pwn.plt("printf").unwrap());

    Ok(())
}

/*

got: []u64 = [
  addr1,   // addr_got1 <- printf
  addr2,   // addr_got2 <- puts
  ...
]

inv_symbols = {
  addr_got1: "printf",
  addr_got2: "puts",
}

// code to jump to the addresses in got is located at plt.
plt = [
  "load ebx, got[0]",  // addr_plt0
  "je ebx 0 0xNNNN",
  "jmp got[0]",
  "load ebx, got[1]",  // addr_plt1
  "je ebx 0 0xNNNN",
  "jmp got[1]",
  "load ebx, got[2]",  // addr_plt2
  "je ebx 0 0xNNNN",
  "jmp got[2]",
  "load ebx, got[3]",
  "je ebx 0 0xNNNN",
  "jmp got[3]",
]

// pwn.plt() returns the address of the instruction as follows:
plt_symbols = {
  "printf": addr_plt0,
  "puts": addr_plt1,
}

*/
