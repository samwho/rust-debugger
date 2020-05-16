#![allow(non_upper_case_globals)]
#![feature(str_strip)]

mod cli;
mod debugger;
mod error;
mod result;
mod sys;
// mod tui;

#[macro_use]
extern crate log;

use crate::cli::Cli;
use crate::debugger::Subordinate;
use crate::error::Error;
use crate::result::Result;
use crate::sys::{disable_aslr, strerror};
use human_panic::setup_panic;
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
    disable_aslr()?;
    let subordinate = Subordinate::spawn(args().skip(1).collect())?;
    let mut cli = Cli::new(subordinate);
    cli.start()?;
    Ok(())
}
