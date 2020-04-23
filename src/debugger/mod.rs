mod dwarf;

use crate::debugger::dwarf::dump_file;
use crate::result::Result;
use crate::sys::{Fork::*, WaitStatus::*, *};
use gimli::constants::{
    DW_AT_high_pc, DW_AT_low_pc, DW_AT_name, DW_TAG_compile_unit, DW_TAG_subprogram,
};
use gimli::read::AttributeValue;
use libc::user_regs_struct;
use object::{Object, ObjectSection};
use std::collections::HashMap;
use std::{borrow, fs, path};

pub struct Subordinate {
    pid: i32,
    registers: Registers,
    wait_status: WaitStatus,
    breakpoints: HashMap<usize, usize>,
    symbols: HashMap<String, (u64, u64)>,
}

impl Subordinate {
    pub fn spawn(cmd: Vec<String>) -> Result<Self> {
        info!("spawning with cmd: {:?}", cmd);

        let pid = match fork()? {
            Parent(child_pid) => child_pid,
            Child => {
                ptrace::traceme()?;
                execvp(&cmd)?;
                0
            }
        };

        let symbols = if std::path::Path::new(&cmd[0]).exists() {
            let file = fs::File::open(&cmd[0]).unwrap();
            Self::parse_debug_info(file)?
        } else {
            HashMap::new()
        };

        Ok(Subordinate {
            pid,
            wait_status: wait()?,
            registers: ptrace::getregs(pid)?.into(),
            breakpoints: HashMap::new(),
            symbols,
        })
    }

    pub fn step(&mut self) -> Result<()> {
        ptrace::singlestep(self.pid)?;
        self.fetch_state()?;
        Ok(())
    }

    pub fn cont(&mut self) -> Result<()> {
        ptrace::cont(self.pid)?;
        self.fetch_state()?;
        Ok(())
    }

    pub fn peek(&self, addr: usize) -> Result<usize> {
        ptrace::peek(self.pid, addr)
    }

    pub fn poke(&self, addr: usize, data: usize) -> Result<()> {
        ptrace::poke(self.pid, addr, data)
    }

    pub fn read_mem(&self, from: usize, size: usize) -> Result<Vec<u8>> {
        let mut bytes = Vec::with_capacity(size);
        let wordlen = std::mem::size_of::<usize>();
        for i in 0..(size / wordlen) {
            for byte in self.peek(from + wordlen * i)?.to_ne_bytes().iter() {
                bytes.push(*byte);
            }
        }
        Ok(bytes)
    }

    pub fn breakpoint(&mut self, addr: usize) -> Result<()> {
        if let Some(_) = self.breakpoints.get(&addr) {
            return Ok(());
        }

        let data = self.peek(addr)?;
        self.poke(addr, data & (usize::max_value() - 255) | 0xcc)?;
        self.breakpoints.insert(addr, data);
        Ok(())
    }

    pub fn registers(&self) -> &Registers {
        &self.registers
    }

    pub fn symbols(&self) -> &HashMap<String, (u64, u64)> {
        &self.symbols
    }

    pub fn exit_status(&self) -> Option<i32> {
        if let Exited(_, exit_status) = self.wait_status {
            return Some(exit_status);
        }
        return None;
    }

    fn fetch_state(&mut self) -> Result<()> {
        self.wait_status = wait()?;
        if let Stopped(_, _) = self.wait_status {
            self.registers = ptrace::getregs(self.pid)?.into();
            self.handle_breakpoint()?;
        };
        Ok(())
    }

    fn handle_breakpoint(&mut self) -> Result<()> {
        let addr = (self.registers.rip - 1) as usize;
        if let Some(data) = self.breakpoints.remove(&addr) {
            info!("hit breakpoint: {:x}", addr);
            self.registers.rip = addr as u64;
            self.poke(self.registers.rip as usize, data)?;
            ptrace::setregs(self.pid, &self.registers.clone().into())?;
        }

        Ok(())
    }

    fn parse_debug_info(file: fs::File) -> Result<HashMap<String, (u64, u64)>> {
        let mmap = unsafe { memmap::Mmap::map(&file).unwrap() };
        let object = object::File::parse(&*mmap).unwrap();
        let endian = if object.is_little_endian() {
            gimli::RunTimeEndian::Little
        } else {
            gimli::RunTimeEndian::Big
        };
        // Load a section and return as `Cow<[u8]>`.
        let load_section =
            |id: gimli::SectionId| -> std::result::Result<borrow::Cow<[u8]>, gimli::Error> {
                match object.section_by_name(id.name()) {
                    Some(ref section) => Ok(section
                        .uncompressed_data()
                        .unwrap_or(borrow::Cow::Borrowed(&[][..]))),
                    None => Ok(borrow::Cow::Borrowed(&[][..])),
                }
            };
        // Load a supplementary section. We don't have a supplementary object file,
        // so always return an empty slice.
        let load_section_sup = |_| Ok(borrow::Cow::Borrowed(&[][..]));

        // Load all of the sections.
        let dwarf_cow = gimli::Dwarf::load(&load_section, &load_section_sup)?;

        // Borrow a `Cow<[u8]>` to create an `EndianSlice`.
        let borrow_section: &dyn for<'a> Fn(
            &'a borrow::Cow<[u8]>,
        )
            -> gimli::EndianSlice<'a, gimli::RunTimeEndian> =
            &|section| gimli::EndianSlice::new(&*section, endian);

        // Create `EndianSlice`s for all of the sections.
        let dwarf = dwarf_cow.borrow(&borrow_section);

        // Iterate over the compilation units.
        let mut iter = dwarf.units();
        let mut symbols: HashMap<String, (u64, u64)> = HashMap::new();
        while let Some(header) = iter.next()? {
            let unit = dwarf.unit(header)?;

            // Iterate over the Debugging Information Entries (DIEs) in the unit.
            let mut entries = unit.entries();
            while let Some((_, entry)) = entries.next_dfs()? {
                match entry.tag() {
                    DW_TAG_subprogram => {
                        let name = match entry.attr(DW_AT_name)? {
                            Some(name) => name
                                .string_value(&dwarf.debug_str)
                                .map(|ds| ds.to_string())
                                .unwrap()?,
                            None => continue,
                        };

                        let low_pc = match entry.attr_value(DW_AT_low_pc)? {
                            Some(AttributeValue::Addr(low_pc)) => low_pc,
                            _ => continue,
                        };

                        let high_pc = match entry.attr_value(DW_AT_high_pc)? {
                            Some(AttributeValue::Udata(high_pc)) => high_pc,
                            _ => continue,
                        };

                        symbols.insert(name.to_string(), (low_pc, high_pc));
                    }
                    DW_TAG_compile_unit => {}
                    _ => {}
                }
            }
        }
        Ok(symbols)
    }
}

#[derive(Clone, Default, Debug)]
pub struct Registers {
    pub r15: u64,
    pub r14: u64,
    pub r13: u64,
    pub r12: u64,
    pub rbp: u64,
    pub rbx: u64,
    pub r11: u64,
    pub r10: u64,
    pub r9: u64,
    pub r8: u64,
    pub rax: u64,
    pub rcx: u64,
    pub rdx: u64,
    pub rsi: u64,
    pub rdi: u64,
    pub orig_rax: u64,
    pub rip: u64,
    pub cs: u64,
    pub eflags: u64,
    pub rsp: u64,
    pub ss: u64,
    pub fs_base: u64,
    pub gs_base: u64,
    pub ds: u64,
    pub es: u64,
    pub fs: u64,
    pub gs: u64,
}

impl From<user_regs_struct> for Registers {
    fn from(r: user_regs_struct) -> Self {
        Registers {
            r15: r.r15,
            r14: r.r14,
            r13: r.r13,
            r12: r.r12,
            rbp: r.rbp,
            rbx: r.rbx,
            r11: r.r11,
            r10: r.r10,
            r9: r.r9,
            r8: r.r8,
            rax: r.rax,
            rcx: r.rcx,
            rdx: r.rdx,
            rsi: r.rsi,
            rdi: r.rdi,
            orig_rax: r.orig_rax,
            rip: r.rip,
            cs: r.cs,
            eflags: r.eflags,
            rsp: r.rsp,
            ss: r.ss,
            fs_base: r.fs_base,
            gs_base: r.gs_base,
            ds: r.ds,
            es: r.es,
            fs: r.fs,
            gs: r.gs,
        }
    }
}

impl From<Registers> for user_regs_struct {
    fn from(r: Registers) -> Self {
        user_regs_struct {
            r15: r.r15,
            r14: r.r14,
            r13: r.r13,
            r12: r.r12,
            rbp: r.rbp,
            rbx: r.rbx,
            r11: r.r11,
            r10: r.r10,
            r9: r.r9,
            r8: r.r8,
            rax: r.rax,
            rcx: r.rcx,
            rdx: r.rdx,
            rsi: r.rsi,
            rdi: r.rdi,
            orig_rax: r.orig_rax,
            rip: r.rip,
            cs: r.cs,
            eflags: r.eflags,
            rsp: r.rsp,
            ss: r.ss,
            fs_base: r.fs_base,
            gs_base: r.gs_base,
            ds: r.ds,
            es: r.es,
            fs: r.fs,
            gs: r.gs,
        }
    }
}
