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
        ["r"] | ["regs"] => print_registers(subordinate)?,
        ["s"] | ["step"] => subordinate.step()?,
        ["c"] | ["cont"] => subordinate.cont()?,
        ["d"] | ["disas"] => {
            let rip = subordinate.registers().rip;
            let bytes = subordinate.read_bytes(rip as usize, 64)?;
            let disassembly = Disassembler::new().disassemble(rip, &bytes)?;
            println!("{}", disassembly);
        }
        ["d", sym] | ["disas", sym] => {
            match subordinate.debug_info().symbol(sym) {
                Some(symbol) => {
                    let rip = symbol.low_pc as u64;
                    let bytes = subordinate.instructions(symbol)?;
                    let disassembly = Disassembler::new().disassemble(rip, &bytes)?;
                    println!("{}", disassembly);
                }
                None => {
                    println!("unknwon symbol {}", sym);
                }
            };
        }
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

fn print_symbols(subordinate: &mut Subordinate) -> Result<()> {
    for (name, symbol) in subordinate.debug_info().symbols().into_iter() {
        println!("0x{:x} {}", symbol.low_pc, name);
    }
    Ok(())
}
