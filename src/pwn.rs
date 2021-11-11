//! ## Example
//!
//! ```no_run
//! use pwntools::pwn::*;
//!
//! let pwn = Pwn::new("./some_elf");
//! println!("name: readn, addr: {:x}", pwn.symbol("readn").unwrap());
//! println!(
//!     "name: __stack_chk_fail, addr: {:x}",
//!     pwn.got("__stack_chk_fail").unwrap()
//! );
//! println!("bss: {:#08x}", pwn.bss().unwrap());
//! ```

use elf_utilities::{
    file, section,
    section::{Contents64, Section64},
};
use unicorn_engine::unicorn_const::{Arch, HookType, Mode, Permission};

// use num_derive::{FromPrimitive, ToPrimitive};
// use num_traits::FromPrimitive;

// #[derive(FromPrimitive, ToPrimitive, PartialEq, Debug)]
// pub enum StBind {
//     Local = 0,
//     Global = 1,
//     Weak = 2,
//     LoProc = 13,
//     HiProc = 15,
// }

// #[derive(FromPrimitive, ToPrimitive, PartialEq, Debug)]
// pub enum StType {
//     NoType = 0,
//     Object = 1,
//     Func = 2,
//     Section = 3,
//     File = 4,
//     LoProc = 13,
//     HiProc = 15,
// }

// fn read_st_info(st_info: u8) -> (StType, StBind) {
//     (
//         StType::from_u8(st_info & 0xF).expect(&format!("StType: {}", st_info & 0xF)),
//         StBind::from_u8((st_info >> 4) & 0xF).expect(&format!("StBind: {}", st_info >> 4 & 0xF)),
//     )
// }

// #[cfg(test)]
// mod tests {
//     use super::*;

//     #[test]
//     fn good_st_info() {
//         assert_eq!(read_st_info(0x12), (StType::Func, StBind::Global));
//     }

//     #[test]
//     #[should_panic(expected = "StBind: 3")]
//     fn bad_st_info() {
//         read_st_info(0x32);
//     }
// }

pub struct Pwn {
    /// The ELF binary being processed.
    pub elf: file::ELF64,
}

impl Pwn {
    pub fn new(file_path: &str) -> Self {
        let elf = elf_utilities::parser::read_elf64(file_path).unwrap();
        Self { elf }
    }

    /// Returns the address of the symbol.
    pub fn symbol(&self, name: &str) -> Option<u64> {
        let mut result = Vec::new();
        for section in &self.elf.sections {
            if let Contents64::Symbols(data) = &section.contents {
                for symbol in data {
                    match &symbol.symbol_name {
                        Some(st_name) if st_name == name => result.push(symbol.st_value),
                        _ => continue,
                    }
                }
            }
        }

        assert!(result.len() <= 1);
        result.first().copied()
    }

    // See: https://github.com/Gallopsled/pwntools/blob/dev/pwnlib/elf/plt.py#L18
    pub fn plt(&self, name: &str) -> Option<u64> {
        let plt = self.get_section(".plt")?;
        dbg!(plt.header.sh_addr);

        let v: &Vec<u8> = match &plt.contents {
            Contents64::Raw(data) => {
                dbg!(data.len());
                data
            }
            _ => unreachable!(),
        };

        let mut unicorn =
            unicorn_engine::Unicorn::new(Arch::X86, Mode::LITTLE_ENDIAN | Mode::MODE_64).unwrap();
        let mut emu = unicorn.borrow();

        let address = plt.header.sh_addr;
        let start = address & (!0xfff);
        let stop = (address + v.len() as u64 + 0xfff) & (!0xfff);
        emu.mem_map(start, (stop - start) as usize, Permission::ALL)
            .unwrap();
        emu.mem_write(plt.header.sh_addr, v).unwrap();

        let mut buf: Vec<u8> = vec![0; v.len()];
        emu.mem_read(plt.header.sh_addr, &mut buf).unwrap();
        assert_eq!(v, &buf);

        let addr = std::rc::Rc::new(std::cell::Cell::new(None));
        {
            let addr = addr.clone();

            emu.add_mem_hook(
                HookType::MEM_READ_UNMAPPED,
                /*begin=*/ u64::MIN,
                /*end=*/ u64::MAX,
                move |mut uc, mem_type, address, size, value| {
                    dbg!((mem_type, address, size, value));
                    addr.set(Some(address));
                    uc.emu_stop().unwrap();
                },
            )
            .unwrap();
        }

        let target_got = dbg!(self.got(name)?);
        let mut pc = address;
        while pc < stop {
            addr.set(None);
            let _ = emu.emu_start(pc, address + v.len() as u64, 100, 5);
            if addr.get() == Some(target_got) {
                return Some(pc);
            }
            pc += 4;
        }
        None
    }

    /// Search the symbol's address in the Global Offset Table.
    pub fn got(&self, name: &str) -> Option<u64> {
        let rela_plt = self.get_section(".rela.plt")?;

        // .dynsym
        let sym_table = &self.elf.sections.get(rela_plt.header.sh_link as usize)?;

        if let section::Contents64::RelaSymbols(data) = &rela_plt.contents {
            data.iter()
                .find(|rela| {
                    let index = rela.get_sym();
                    if let section::Contents64::Symbols(data) = &sym_table.contents {
                        if let Some(st_name) = &data[index as usize].symbol_name {
                            st_name == name
                        } else {
                            false
                        }
                    } else {
                        false
                    }
                })
                .map(|rela| rela.get_offset())
        } else {
            None
        }
    }

    /// The beginning of the bss section.
    pub fn bss(&self) -> Option<u64> {
        let bss = self.get_section(".bss")?;
        Some(bss.header.sh_addr)
    }

    fn get_section(&self, name: &str) -> Option<&Section64> {
        self.elf
            .sections
            .iter()
            .find(|section| section.name == name)
    }
}
