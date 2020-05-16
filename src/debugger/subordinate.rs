use crate::debugger::{
    auxv::{self, Entry::*},
    DebugInfo, Registers,
};

use crate::result::Result;
use crate::sys::{Fork::*, WaitStatus::*, *};
use std::collections::HashMap;
use std::fs::File;

use elf;

pub struct Subordinate {
    pid: i32,
    registers: Registers,
    stack: Vec<usize>,
    wait_status: WaitStatus,
    breakpoints: HashMap<usize, usize>,
    debug_info: DebugInfo,
    auxv: Vec<auxv::Entry>,
    symbols: Vec<elf::types::Symbol>,
}

impl Subordinate {
    pub fn spawn(cmd: Vec<String>) -> Result<Self> {
        if cmd.len() == 0 {
            return Err("empty command given".into());
        }

        info!("spawning with cmd: {:?}", cmd);

        let pid = match fork()? {
            Parent(child_pid) => child_pid,
            Child => {
                ptrace::traceme()?;
                execvp(&cmd)?;
                0
            }
        };

        let elf = elf::File::open_path(&cmd[0])?;
        let debug_info = DebugInfo::new(File::open(&cmd[0])?)?;

        let mut symbols: Vec<elf::types::Symbol> = Vec::new();
        if let Some(section) = elf.get_section(".symtab") {
            symbols = elf.get_symbols(section)?;
        }

        let mut subordinate = Subordinate {
            pid,
            wait_status: WaitStatus::Unknwon(0, 0),
            registers: Registers::default(),
            stack: Vec::new(),
            breakpoints: HashMap::new(),
            debug_info,
            auxv: Vec::new(),
            symbols,
        };

        subordinate.fetch_state()?;

        let auxv = auxv::read(&subordinate)?;
        for entry in &auxv {
            match entry {
                EntryAddr(addr) => {
                    let amount = *addr as u64 - elf.ehdr.entry;
                    subordinate.shift_symbols(amount);
                    break;
                }
                _ => {}
            }
        }

        subordinate.auxv = auxv;

        Ok(subordinate)
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

    pub fn read_bytes(&self, from: usize, size: usize) -> Result<Vec<u8>> {
        let mut bytes = Vec::with_capacity(size);
        let wordlen = std::mem::size_of::<usize>();
        for i in 0..(size / wordlen) + 1 {
            for byte in self.peek(from + wordlen * i)?.to_ne_bytes().iter() {
                bytes.push(*byte);
                if bytes.len() == size {
                    break;
                }
            }
        }
        Ok(bytes)
    }

    pub fn read_words(&self, from: usize, size: usize) -> Result<Vec<usize>> {
        let mut words = Vec::with_capacity(size);
        let wordlen = std::mem::size_of::<usize>();
        for i in 0..size {
            words.push(self.peek(from + wordlen * i)?);
        }
        Ok(words)
    }

    pub fn exit_status(&self) -> Option<i32> {
        if let Exited(_, status) = self.wait_status {
            return Some(status);
        }
        None
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

    pub fn instructions(&self, symbol: &elf::types::Symbol) -> Result<Vec<u8>> {
        Ok(self.read_bytes(symbol.value as usize, symbol.size as usize)?)
    }

    pub fn stack(&self) -> &[usize] {
        &self.stack
    }

    pub fn debug_info(&self) -> &DebugInfo {
        &self.debug_info
    }

    pub fn symbols(&self) -> &Vec<elf::types::Symbol> {
        &self.symbols
    }

    pub fn symbol(&self, name: &str) -> Option<&elf::types::Symbol> {
        for symbol in &self.symbols {
            if symbol.name == name {
                return Some(symbol);
            }
        }
        None
    }

    fn shift_symbols(&mut self, amount: u64) {
        for symbol in &mut self.symbols {
            if symbol.bind == elf::types::STB_WEAK {
                continue;
            }
            symbol.value += amount;
        }
    }

    fn fetch_state(&mut self) -> Result<()> {
        self.wait_status = wait()?;
        if let Stopped(_, _) = self.wait_status {
            self.registers = ptrace::getregs(self.pid)?.into();
            self.stack = self.read_words(self.registers.rsp as usize, 16)?;
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
}
