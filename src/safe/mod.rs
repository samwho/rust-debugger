use crate::result::Result;
use libc::{pid_t, fork as libcfork};

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