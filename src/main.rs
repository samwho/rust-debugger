mod debugger;
mod error;
mod result;
mod sys;

#[macro_use]
extern crate log;

#[macro_use]
extern crate lazy_static;

use crate::debugger::Subordinate;
use crate::error::Error;
use crate::result::Result;
use crate::sys::strerror;
use human_panic::setup_panic;
use rustyline::error::ReadlineError;
use rustyline::Editor;
use std::collections::HashMap;
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
    if rl.load_history("history.txt").is_err() {}
    loop {
        let readline = rl.readline("> ");
        match readline {
            Ok(line) => {
                rl.add_history_entry(line.as_str());
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
    rl.save_history("history.txt")?;

    Ok(())
}

fn execute_command(subordinate: &mut Subordinate, cmd: Vec<&str>) -> Result<()> {
    match cmd.as_slice() {
        ["regs"] => println!("{:?}", subordinate.registers()),
        ["step"] => subordinate.step()?,
        ["cont"] => subordinate.cont()?,
        other => println!("unknown command `{:?}`", other),
    };

    Ok(())
}
