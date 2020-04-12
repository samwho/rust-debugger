mod error;
mod result;
mod safe;

use crate::error::Error;
use crate::result::Result;
use crate::safe::{
    execl, fork, ptrace, strerror, wait,
    Fork::{Child, Parent},
    WaitStatus::Stopped,
};
use human_panic::setup_panic;
use libc::pid_t;
use std::process::exit;

fn main() {
    setup_panic!();

    match app() {
        Ok(_) => {}
        Err(Error::Errno(errno)) => {
            let errstr = strerror(errno).unwrap();
            eprintln!("libc err: {}", errstr);
            exit(1);
        }
        Err(e) => {
            eprintln!("err: {}", e);
            exit(1);
        }
    }
}

fn app() -> Result<()> {
    match fork()? {
        Parent(child_pid) => parent(child_pid)?,
        Child => child()?,
    };

    Ok(())
}

fn parent(child_pid: pid_t) -> Result<()> {
    println!("[parent] child_pid {}", child_pid);

    let mut icounter = 0;
    loop {
        match wait()? {
            Stopped(_, _) => {
                icounter += 1;
                let regs = ptrace::getregs(child_pid)?;
                println!("rax: {}", regs.rax);
                ptrace::singlestep(child_pid)?;
            }
            _ => break,
        }
    }

    println!("[parent] child executed {} instructions", icounter);

    Ok(())
}

fn child() -> Result<()> {
    println!("[child] calling traceme");
    ptrace::traceme()?;

    println!("[child] executing binary");
    execl("/bin/date")?;
    Ok(())
}
