mod dwarf;

use crate::debugger::dwarf::DebugInfo;
use crate::result::Result;
use crate::sys::{Fork::*, WaitStatus::*, *};
use libc::user_regs_struct;
use std::collections::HashMap;
use std::fs::File;

pub struct Subordinate {
    pid: i32,
    registers: Registers,
    stack: Vec<usize>,
    instructions: Vec<u8>,
    wait_status: WaitStatus,
    breakpoints: HashMap<usize, usize>,
    debug_info: DebugInfo,
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

        let debug_info = DebugInfo::new(File::open(&cmd[0])?)?;

        let mut subordinate = Subordinate {
            pid,
            wait_status: WaitStatus::Unknwon(0, 0),
            registers: Registers::default(),
            stack: Vec::new(),
            instructions: Vec::new(),
            breakpoints: HashMap::new(),
            debug_info,
        };

        subordinate.fetch_state()?;
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
        for i in 0..(size / wordlen) {
            for byte in self.peek(from + wordlen * i)?.to_ne_bytes().iter() {
                bytes.push(*byte);
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

    pub fn instructions(&self) -> &[u8] {
        &self.instructions
    }

    pub fn stack(&self) -> &[usize] {
        &self.stack
    }

    pub fn debug_info(&self) -> &DebugInfo {
        &self.debug_info
    }

    pub fn exit_status(&self) -> Option<i32> {
        if let Exited(_, exit_status) = self.wait_status {
            return Some(exit_status);
        }
        None
    }

    fn fetch_state(&mut self) -> Result<()> {
        self.wait_status = wait()?;
        if let Stopped(_, _) = self.wait_status {
            self.registers = ptrace::getregs(self.pid)?.into();
            self.stack = self.read_words(self.registers.rsp as usize, 16)?;
            self.instructions = self.read_bytes(self.registers.rip as usize, 64)?;
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
