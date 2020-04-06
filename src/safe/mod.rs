pub mod ptrace;

use crate::result::Result;
use crate::error::Error;
use libc::{__error, pid_t, fork as libcfork, execl as libcexecl};
use std::ffi::{CString};

pub enum Fork {
  Parent(pid_t), Child
}

pub fn fork() -> Result<Fork> {
  match unsafe { libcfork() } {
    errno if errno < 0 => Err("error".into()),
    0 => Ok(Fork::Child),
    pid => Ok(Fork::Parent(pid)),
  }
}

pub fn errwrap<F, T>(f: F) -> Result<T>
  where F: FnOnce() -> T
{
  unsafe { *__error() = 0 };
  let result = f();
  match unsafe { *__error() } {
    0 => Ok(result),
    errno => Err(Error::Errno(errno)),
  }
}

pub fn execl(progname: &str) -> Result<()> {
  let cstr = CString::new(progname)?;
  errwrap(|| unsafe { libcexecl(cstr.as_ptr(), cstr.as_ptr(), 0) })?;
  Ok(())
}