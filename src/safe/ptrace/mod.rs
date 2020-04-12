use crate::result::Result;
use crate::safe::errwrap;
use libc::{pid_t, ptrace, PTRACE_SINGLESTEP, PTRACE_TRACEME};

pub fn traceme() -> Result<()> {
    errwrap(|| unsafe { ptrace(PTRACE_TRACEME, 0, &mut 0, 0) })?;
    Ok(())
}

pub fn singlestep(pid: pid_t) -> Result<()> {
    errwrap(|| unsafe { ptrace(PTRACE_SINGLESTEP, pid, &mut 0, 0) })?;
    Ok(())
}
