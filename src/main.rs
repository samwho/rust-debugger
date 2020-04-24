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
use iced_x86::{Decoder, DecoderOptions, Formatter, Instruction, NasmFormatter};
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
        ["r"] | ["regs"] => print_registers(subordinate)?,
        ["s"] | ["step"] => subordinate.step()?,
        ["c"] | ["cont"] => subordinate.cont()?,
        ["d"] | ["disas"] => print_disassembly(subordinate)?,
        ["syms"] | ["symbols"] => print_symbols(subordinate)?,
        ["b", addr] | ["break", addr] => set_breakpoint(subordinate, addr)?,
        other => println!("unknown command `{:?}`", other),
    };

    Ok(())
}

fn set_breakpoint(subordinate: &mut Subordinate, addr: &str) -> Result<()> {
    if let Ok(addr) = usize::from_str_radix(addr, 16) {
        return subordinate.breakpoint(addr);
    }

    let symbols = subordinate.debug_info().symbols();
    let fetch = symbols.get(addr).map(|t| t.to_owned());
    if let Some(symbol) = fetch {
        return subordinate.breakpoint(symbol.low_pc as usize);
    }

    Err(format!(
        "couldn't set breakpoint on `{}`, not a known address or symbol",
        addr
    )
    .into())
}

fn print_registers(subordinate: &mut Subordinate) -> Result<()> {
    let regs = subordinate.registers();

    println!("rip: 0x{:x}", regs.rip);
    println!("rsp: 0x{:x}", regs.rsp);
    println!("rbp: 0x{:x}", regs.rbp);
    println!("rax: 0x{:x}", regs.rax);
    println!("rbx: 0x{:x}", regs.rbx);
    println!("rcx: 0x{:x}", regs.rcx);
    println!("rdx: 0x{:x}", regs.rdx);
    println!("rdi: 0x{:x}", regs.rdi);
    println!("rsi: 0x{:x}", regs.rsi);

    Ok(())
}

fn print_disassembly(subordinate: &mut Subordinate) -> Result<()> {
    let regs = subordinate.registers();
    let bytes = subordinate.read_mem(regs.rip as usize, 64)?;
    let mut decoder = Decoder::new(64, bytes.as_slice(), DecoderOptions::NONE);
    decoder.set_ip(regs.rip);

    let mut formatter = NasmFormatter::new();

    let mut output = String::new();

    let mut instruction = Instruction::default();

    while decoder.can_decode() {
        decoder.decode_out(&mut instruction);

        output.clear();
        formatter.format(&instruction, &mut output);

        print!("{:016x} ", instruction.ip());
        let start_index = (instruction.ip() - regs.rip) as usize;
        let instr_bytes = &bytes[start_index..start_index + instruction.len()];
        for b in instr_bytes.iter() {
            print!("{:02x}", b);
        }
        if instr_bytes.len() < 10 {
            for _ in 0..10 - instr_bytes.len() {
                print!("  ");
            }
        }
        println!(" {}", output);
    }

    Ok(())
}

fn print_symbols(subordinate: &mut Subordinate) -> Result<()> {
    for (name, symbol) in subordinate.debug_info().symbols().into_iter() {
        println!("0x{:x} {}", symbol.low_pc, name);
    }
    Ok(())
}
