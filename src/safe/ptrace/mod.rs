use crate::result::Result;
use crate::safe::errwrap;
use libc::{pid_t, ptrace, PT_STEP, PT_TRACE_ME};

pub fn traceme() -> Result<()> {
    errwrap(|| unsafe { ptrace(PT_TRACE_ME, 0, &mut 0, 0) })?;
    Ok(())
}

pub fn singlestep(pid: pid_t) -> Result<()> {
    errwrap(|| unsafe { ptrace(PT_STEP, pid, &mut 0, 0) })?;
    Ok(())
}
