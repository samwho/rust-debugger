use crate::result::Result;
use crate::safe::errwrap;
use libc::{pid_t, ptrace, user_regs_struct, PTRACE_GETREGS, PTRACE_SINGLESTEP, PTRACE_TRACEME};

pub fn traceme() -> Result<()> {
    errwrap(|| unsafe { ptrace(PTRACE_TRACEME, 0, &mut 0, 0) })?;
    Ok(())
}

pub fn singlestep(pid: pid_t) -> Result<()> {
    errwrap(|| unsafe { ptrace(PTRACE_SINGLESTEP, pid, &mut 0, 0) })?;
    Ok(())
}

pub fn getregs(pid: pid_t) -> Result<user_regs_struct> {
    let mut regs: user_regs_struct = user_regs_struct {
        r15: 0,
        r14: 0,
        r13: 0,
        r12: 0,
        rbp: 0,
        rbx: 0,
        r11: 0,
        r10: 0,
        r9: 0,
        r8: 0,
        rax: 0,
        rcx: 0,
        rdx: 0,
        rsi: 0,
        rdi: 0,
        orig_rax: 0,
        rip: 0,
        cs: 0,
        eflags: 0,
        rsp: 0,
        ss: 0,
        fs_base: 0,
        gs_base: 0,
        ds: 0,
        es: 0,
        fs: 0,
        gs: 0,
    };

    errwrap(|| unsafe { ptrace(PTRACE_GETREGS, pid, 0, &mut regs) })?;

    Ok(regs)
}
