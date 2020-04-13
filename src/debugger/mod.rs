use crate::result::Result;
use crate::sys::{Fork::*, WaitStatus::*, *};
use libc::user_regs_struct;

#[derive(Clone, Default, Debug)]
pub struct Registers {
    r15: u64,
    r14: u64,
    r13: u64,
    r12: u64,
    rbp: u64,
    rbx: u64,
    r11: u64,
    r10: u64,
    r9: u64,
    r8: u64,
    rax: u64,
    rcx: u64,
    rdx: u64,
    rsi: u64,
    rdi: u64,
    orig_rax: u64,
    rip: u64,
    cs: u64,
    eflags: u64,
    rsp: u64,
    ss: u64,
    fs_base: u64,
    gs_base: u64,
    ds: u64,
    es: u64,
    fs: u64,
    gs: u64,
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

pub struct Subordinate {
    pid: i32,
    registers: Registers,
    wait_status: WaitStatus,
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

        Ok(Subordinate {
            pid,
            wait_status: wait()?,
            registers: ptrace::getregs(pid)?.into(),
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

    pub fn registers(&self) -> &Registers {
        &self.registers
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
        };
        Ok(())
    }
}
