mod debugger;
mod error;
mod result;
mod sys;

#[macro_use]
extern crate log;

use crate::debugger::Subordinate;
use crate::error::Error;
use crate::result::Result;
use crate::sys::strerror;
use human_panic::setup_panic;
use rustyline::error::ReadlineError;
use rustyline::Editor;
use std::env::args;
use std::process::exit;

fn main() {
    env_logger::init();
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
    let mut subordinate = Subordinate::spawn(args().skip(1).collect())?;

    let mut rl = Editor::<()>::new();
    loop {
        let readline = rl.readline("> ");
        match readline {
            Ok(line) => {
                execute_command(&mut subordinate, line.split_whitespace().collect())?;
                if let Some(exit_status) = subordinate.exit_status() {
                    println!("debugged process exited with status: {}", exit_status);
                    break;
                }
            }
            Err(ReadlineError::Interrupted) => break,
            Err(ReadlineError::Eof) => break,
            Err(err) => return Err(err.into()),
        }
    }

    Ok(())
}

fn execute_command(subordinate: &mut Subordinate, cmd: Vec<&str>) -> Result<()> {
    match cmd.as_slice() {
        ["regs"] => println!("{:?}", subordinate.registers()),
        ["step"] => subordinate.step()?,
        ["cont"] => subordinate.cont()?,
        ["break", addr] => subordinate.breakpoint(usize::from_str_radix(addr, 16)?)?,
        other => println!("unknown command `{:?}`", other),
    };

    Ok(())
}
