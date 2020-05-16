use crate::debugger::{Disassembler, Subordinate};
use crate::result::Result;
use rustyline::error::ReadlineError;
use rustyline::Editor;

pub struct Cli {
    subordinate: Subordinate,
}

impl Cli {
    pub fn new(subordinate: Subordinate) -> Self {
        Self { subordinate }
    }

    pub fn start(&mut self) -> Result<()> {
        let mut rl = Editor::<()>::new();
        loop {
            let readline = rl.readline("> ");
            match readline {
                Ok(line) => {
                    execute_command(&mut self.subordinate, line.split_whitespace().collect())?;
                    if let Some(exit_status) = self.subordinate.exit_status() {
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
}

fn execute_command(subordinate: &mut Subordinate, cmd: Vec<&str>) -> Result<()> {
    match cmd.as_slice() {
        ["regs"] | ["registers"] => print_registers(subordinate)?,
        ["r", name] | ["reg", name] | ["register", name] => print_register(subordinate, name)?,
        ["si"] | ["stepi"] => subordinate.step()?,
        ["c"] | ["cont"] => subordinate.cont()?,
        ["d"] | ["disas"] => {
            let rip = subordinate.registers().rip;
            let bytes = subordinate.read_bytes(rip as usize, 64)?;
            let disassembly = Disassembler::new().disassemble(rip, &bytes)?;
            println!("{}", disassembly);
        }
        ["d", sym] | ["disas", sym] => {
            match subordinate.symbol(sym) {
                Some(symbol) => {
                    let rip = symbol.value;
                    let bytes = subordinate.instructions(symbol)?;
                    let disassembly = Disassembler::new().disassemble(rip, &bytes)?;
                    println!("{}", disassembly);
                }
                None => {
                    println!("unknwon symbol {}", sym);
                }
            };
        }
        ["l", sym] | ["list", sym] => {
            let debug_info = subordinate.debug_info();
            let lines = subordinate
                .symbol(sym)
                .and_then(|symbol| debug_info.line_info(symbol.value as usize))
                .and_then(|line_info| debug_info.lines(&line_info.path));

            if let Some(lines) = lines {
                lines.iter().for_each(|line| println!("{}", line));
            } else {
                println!("couldn't find source code for symbol {}", sym);
            }
        }
        ["syms"] | ["symbols"] => print_symbols(subordinate)?,
        ["sym", name] | ["symbol", name] => print_symbol(subordinate, name)?,
        ["b", addr] | ["break", addr] => set_breakpoint(subordinate, addr)?,
        other => println!("unknown command `{:?}`", other),
    };

    Ok(())
}

fn set_breakpoint(subordinate: &mut Subordinate, addr: &str) -> Result<()> {
    if let Some(hex) = addr.strip_prefix("0x") {
        if let Ok(addr) = usize::from_str_radix(hex, 16) {
            return subordinate.breakpoint(addr);
        }
    }

    let name = addr;
    if let Some(symbol) = subordinate.symbol(name).map(|s| s.to_owned()) {
        return subordinate.breakpoint(symbol.value as usize);
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

fn print_register(subordinate: &mut Subordinate, name: &str) -> Result<()> {
    match subordinate.registers().get(name) {
        Some(value) => {
            println!("{} 0x{:x}", name, value);
        }
        None => {
            println!("couldn't find register with name \"{}\"", name);
        }
    }
    Ok(())
}

fn print_symbols(subordinate: &mut Subordinate) -> Result<()> {
    for symbol in subordinate.symbols().into_iter() {
        if symbol.symtype != elf::types::STT_FUNC {
            continue;
        }
        println!("0x{:x} {}", symbol.value, symbol.name);
    }
    Ok(())
}

fn print_symbol(subordinate: &mut Subordinate, name: &str) -> Result<()> {
    for symbol in subordinate.symbols() {
        if symbol.name != name {
            continue;
        }
        println!("0x{:x} {}", symbol.value, symbol.name);
        return Ok(());
    }
    println!("couldn't find symbol with name \"{}\"", name);
    Ok(())
}
