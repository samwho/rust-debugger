use libc::{ptrace, PT_TRACE_ME};
use crate::safe::errwrap;
use crate::result::Result;

pub fn traceme() -> Result<()> {
  errwrap(|| unsafe { ptrace(PT_TRACE_ME, 0, &mut 0, 0) })?;
  Ok(())
}