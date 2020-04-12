pub mod ptrace;

use crate::error::Error;
use crate::result::Result;
use libc::{
    __errno_location, c_int, execl as libcexecl, fork as libcfork, pid_t, strerror as libcstrerror,
    wait as libcwait, WEXITSTATUS, WIFCONTINUED, WIFEXITED, WIFSIGNALED, WIFSTOPPED, WSTOPSIG,
    WTERMSIG,
};
use std::ffi::CString;

pub enum Fork {
    Parent(pid_t),
    Child,
}

pub fn fork() -> Result<Fork> {
    match unsafe { libcfork() } {
        errno if errno < 0 => Err(Error::Errno(-errno)),
        0 => Ok(Fork::Child),
        pid => Ok(Fork::Parent(pid)),
    }
}

pub fn errwrap<F, T>(f: F) -> Result<T>
where
    F: FnOnce() -> T,
{
    unsafe { *__errno_location() = 0 };
    let result = f();
    match unsafe { *__errno_location() } {
        0 => Ok(result),
        errno => Err(Error::Errno(errno)),
    }
}

pub fn strerror(errno: c_int) -> Result<String> {
    let str_ptr = errwrap(|| unsafe { libcstrerror(errno) })?;
    let cs = unsafe { CString::from_raw(str_ptr) };
    Ok(cs.into_string()?)
}

pub fn execl(progname: &str) -> Result<()> {
    let cstr = CString::new(progname)?;
    errwrap(|| unsafe { libcexecl(cstr.as_ptr(), cstr.as_ptr(), 0) })?;
    Ok(())
}

pub enum WaitStatus {
    Stopped(pid_t, i32),
    Continued(pid_t),
    Exited(pid_t, i32),
    Signaled(pid_t, i32),
    Unknwon(pid_t, i32),
}

pub fn wait() -> Result<WaitStatus> {
    let mut status = 0;
    let pid = errwrap(|| unsafe { libcwait(&mut status) })?;

    let ws = if unsafe { WIFSTOPPED(status) } {
        let stopsig = unsafe { WSTOPSIG(status) };
        WaitStatus::Stopped(pid, stopsig)
    } else if unsafe { WIFEXITED(status) } {
        let exitstatus = unsafe { WEXITSTATUS(status) };
        WaitStatus::Exited(pid, exitstatus)
    } else if unsafe { WIFCONTINUED(status) } {
        WaitStatus::Continued(pid)
    } else if unsafe { WIFSIGNALED(status) } {
        let termsig = unsafe { WTERMSIG(status) };
        WaitStatus::Signaled(pid, termsig)
    } else {
        WaitStatus::Unknwon(pid, status)
    };

    Ok(ws)
}
