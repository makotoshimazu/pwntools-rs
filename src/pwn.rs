use elf_utilities::{
    file, section,
    section::{Contents64, Section64},
};
use num_derive::{FromPrimitive, ToPrimitive};
use num_traits::FromPrimitive;

#[derive(FromPrimitive, ToPrimitive, PartialEq, Debug)]
pub enum StBind {
    Local = 0,
    Global = 1,
    Weak = 2,
    LoProc = 13,
    HiProc = 15,
}

#[derive(FromPrimitive, ToPrimitive, PartialEq, Debug)]
pub enum StType {
    NoType = 0,
    Object = 1,
    Func = 2,
    Section = 3,
    File = 4,
    LoProc = 13,
    HiProc = 15,
}

pub fn read_st_info(st_info: u8) -> (StType, StBind) {
    (
        StType::from_u8(st_info & 0xF).expect(&format!("StType: {}", st_info & 0xF)),
        StBind::from_u8((st_info >> 4) & 0xF).expect(&format!("StBind: {}", st_info >> 4 & 0xF)),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn good_st_info() {
        assert_eq!(read_st_info(0x12), (StType::Func, StBind::Global));
    }

    #[test]
    #[should_panic(expected = "StBind: 3")]
    fn bad_st_info() {
        read_st_info(0x32);
    }
}

pub struct Pwn {
    pub elf: file::ELF64,
}

impl Pwn {
    pub fn new(file_path: &str) -> Self {
        let elf = elf_utilities::parser::read_elf64(file_path).unwrap();
        Self { elf }
    }

    pub fn symbol(&self, name: &str) -> Option<u64> {
        let mut result = Vec::new();
        for section in &self.elf.sections {
            match &section.contents {
                Contents64::Symbols(data) => {
                    for symbol in data {
                        if let Some(st_name) = &symbol.symbol_name {
                            if st_name == name {
                                result.push(symbol.st_value);
                            }
                        }
                    }
                }
                _ => (),
            }
        }

        assert!(result.len() <= 1);
        result.first().copied()
    }

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
