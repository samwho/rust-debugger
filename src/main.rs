mod error;
mod result;
mod safe;

use crate::error::Error;
use crate::result::Result;
use crate::safe::{
    execl, fork, ptrace, strerror, wait,
    Fork::{Child, Parent},
};
use human_panic::setup_panic;
use libc::pid_t;
use rustyline::error::ReadlineError;
use rustyline::Editor;
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
    wait()?;

    let mut rl = Editor::<()>::new();
    if rl.load_history("history.txt").is_err() {}
    loop {
        let readline = rl.readline("> ");
        match readline {
            Ok(line) => {
                rl.add_history_entry(line.as_str());
                match line.as_str() {
                    "regs" => print_regs(child_pid)?,
                    "step" => ptrace::singlestep(child_pid)?,
                    other => println!("unknown command `{}`", other),
                }
            }
            Err(ReadlineError::Interrupted) => break,
            Err(ReadlineError::Eof) => break,
            Err(err) => return Err(err.into()),
        }
    }
    rl.save_history("history.txt")?;

    Ok(())
}

fn child() -> Result<()> {
    println!("[child] calling traceme");
    ptrace::traceme()?;

    println!("[child] executing binary");
    execl("/bin/date")?;
    Ok(())
}

fn print_regs(pid: pid_t) -> Result<()> {
    let regs = ptrace::getregs(pid)?;

    println!("rip: {:#x}", regs.rip);
    println!("rax: {:#x}", regs.rax);
    println!("rbx: {:#x}", regs.rbx);
    println!("rcx: {:#x}", regs.rcx);
    println!("rdx: {:#x}", regs.rdx);
    println!("rbp: {:#x}", regs.rbp);
    println!("rsp: {:#x}", regs.rsp);

    Ok(())
}
