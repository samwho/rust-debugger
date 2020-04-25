use std::io::{self, Write};
use std::sync::mpsc;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::thread;
use std::time::Duration;

use termion::event::Key;
use termion::input::TermRead;

use termion::{cursor::Goto, input::MouseTerminal, raw::IntoRawMode, screen::AlternateScreen};
use tui::{
    backend::TermionBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Style},
    widgets::{Block, BorderType, Borders, Paragraph, Text},
    Terminal,
};
use unicode_width::UnicodeWidthStr;

use iced_x86::{Decoder, DecoderOptions, Formatter, Instruction, NasmFormatter};

use crate::debugger::Subordinate;
use crate::result::Result;

pub enum Event<I> {
    Input(I),
    Tick,
}

/// A small event handler that wrap termion input and tick events. Each event
/// type is handled in its own thread and returned to a common `Receiver`
#[allow(dead_code)]
pub struct Events {
    rx: mpsc::Receiver<Event<Key>>,
    input_handle: thread::JoinHandle<()>,
    ignore_exit_key: Arc<AtomicBool>,
    tick_handle: thread::JoinHandle<()>,
}

#[derive(Debug, Clone, Copy)]
pub struct Config {
    pub exit_key: Key,
    pub tick_rate: Duration,
}

impl Default for Config {
    fn default() -> Config {
        Config {
            exit_key: Key::Char('q'),
            tick_rate: Duration::from_millis(250),
        }
    }
}

impl Events {
    pub fn new() -> Events {
        Events::with_config(Config::default())
    }

    pub fn with_config(config: Config) -> Events {
        let (tx, rx) = mpsc::channel();
        let ignore_exit_key = Arc::new(AtomicBool::new(false));
        let input_handle = {
            let tx = tx.clone();
            let ignore_exit_key = ignore_exit_key.clone();
            thread::spawn(move || {
                let stdin = io::stdin();
                for evt in stdin.keys() {
                    match evt {
                        Ok(key) => {
                            if let Err(_) = tx.send(Event::Input(key)) {
                                return;
                            }
                            if !ignore_exit_key.load(Ordering::Relaxed) && key == config.exit_key {
                                return;
                            }
                        }
                        Err(_) => {}
                    }
                }
            })
        };
        let tick_handle = {
            let tx = tx.clone();
            thread::spawn(move || {
                let tx = tx.clone();
                loop {
                    tx.send(Event::Tick).unwrap();
                    thread::sleep(config.tick_rate);
                }
            })
        };
        Events {
            rx,
            ignore_exit_key,
            input_handle,
            tick_handle,
        }
    }

    pub fn next(&self) -> Result<Event<Key>> {
        Ok(self.rx.recv()?)
    }
}

pub struct Tui {
    input: String,
    program_output: Vec<u8>,
    command_output: Vec<u8>,
    subordinate: Subordinate,
}

impl Tui {
    pub fn new(subordinate: Subordinate) -> Self {
        Self {
            input: String::new(),
            program_output: Vec::new(),
            command_output: Vec::new(),
            subordinate,
        }
    }

    pub fn start(&mut self) -> Result<()> {
        // Terminal initialization
        let stdout = io::stdout().into_raw_mode()?;
        let stdout = MouseTerminal::from(stdout);
        let stdout = AlternateScreen::from(stdout);
        let backend = TermionBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;

        // Setup event handlers
        let events = Events::new();

        loop {
            // Draw UI
            terminal.draw(|mut f| {
                let chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints(
                        [
                            Constraint::Min(1),
                            Constraint::Length(6),
                            Constraint::Length(3),
                        ]
                        .as_ref(),
                    )
                    .split(f.size());

                let top = chunks[0];
                let middle = chunks[1];
                let bottom = chunks[2];

                let top_chunks = Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints(
                        [
                            Constraint::Percentage(20),
                            Constraint::Percentage(60),
                            Constraint::Percentage(20),
                        ]
                        .as_ref(),
                    )
                    .split(top);

                let bottom_chunks = Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints([Constraint::Percentage(50), Constraint::Percentage(50)].as_ref())
                    .split(middle);

                let left = top_chunks[0];
                let middle = top_chunks[1];
                let right = top_chunks[2];

                let bottom_left = bottom_chunks[0];
                let bottom_right = bottom_chunks[1];

                let block = Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(Style::default().fg(Color::DarkGray));

                let registers_s = match registers(&self.subordinate) {
                    Ok(s) => s,
                    Err(e) => {
                        write!(&mut self.command_output, "{}", e).unwrap();
                        "".to_owned()
                    }
                };

                let left_text = [Text::raw(registers_s)];
                let left_para =
                    Paragraph::new(left_text.iter()).block(block.clone().title("Registers"));
                f.render_widget(left_para, left);

                let disassembly_s = match disassemble(&self.subordinate) {
                    Ok(s) => s,
                    Err(e) => {
                        write!(&mut self.command_output, "{}", e).unwrap();
                        "".to_owned()
                    }
                };

                let middle_text = [Text::raw(disassembly_s)];
                let middle_para =
                    Paragraph::new(middle_text.iter()).block(block.clone().title("Disassembly"));
                f.render_widget(middle_para, middle);

                let stack_s = match stack(&self.subordinate) {
                    Ok(s) => s,
                    Err(e) => {
                        write!(&mut self.command_output, "{}", e).unwrap();
                        "".to_owned()
                    }
                };

                let right_text = [Text::raw(stack_s)];
                let right_para =
                    Paragraph::new(right_text.iter()).block(block.clone().title("Stack"));
                f.render_widget(right_para, right);

                let bottom_left_text = [Text::raw(String::from_utf8_lossy(&self.command_output))];
                let bottom_left_para = Paragraph::new(bottom_left_text.iter())
                    .wrap(true)
                    .block(block.clone().title("Command output"));
                f.render_widget(bottom_left_para, bottom_left);

                let bottom_right_text = [Text::raw(String::from_utf8_lossy(&self.program_output))];
                let bottom_right_para = Paragraph::new(bottom_right_text.iter())
                    .wrap(true)
                    .block(block.clone().title("Program output"));
                f.render_widget(bottom_right_para, bottom_right);

                let text = [Text::raw(&self.input)];
                let input = Paragraph::new(text.iter())
                    .style(Style::default().fg(Color::Yellow))
                    .block(block.clone().title("Prompt"));
                f.render_widget(input, bottom);
            })?;

            let termsize = terminal.size()?;
            // Put the cursor back inside the input box
            write!(
                terminal.backend_mut(),
                "{}",
                Goto(2 + self.input.width() as u16, termsize.height - 1)
            )?;
            // stdout is buffered, flush it to see the effect immediately when hitting backspace
            io::stdout().flush().ok();

            // Handle input
            match events.next()? {
                Event::Input(input) => match input {
                    Key::Char('\n') => {
                        let cmd: String = self.input.drain(..).collect();
                        if let Err(e) =
                            execute_command(&mut self.subordinate, cmd.split_whitespace().collect())
                        {
                            writeln!(&mut self.command_output, "{}", e)?;
                        }
                    }
                    Key::Char(c) => {
                        self.input.push(c);
                    }
                    Key::Backspace => {
                        self.input.pop();
                    }
                    Key::Esc => {
                        break;
                    }
                    _ => {}
                },
                _ => {}
            }
        }
        Ok(())
    }
}

fn disassemble(subordinate: &Subordinate) -> Result<String> {
    let regs = subordinate.registers();
    let debug_info = subordinate.debug_info();
    let symbol = debug_info.symbol_for_pc(regs.rip as usize);

    let (rip, bytes) = match symbol {
        Some(symbol) => (symbol.low_pc as u64, subordinate.instructions(symbol)?),
        None => (regs.rip, subordinate.read_bytes(regs.rip as usize, 64)?),
    };

    let mut decoder = Decoder::new(64, &bytes, DecoderOptions::NONE);
    decoder.set_ip(rip);

    let mut formatter = NasmFormatter::new();
    let mut ret: Vec<u8> = Vec::new();
    let mut buf = String::new();
    let mut instruction = Instruction::default();

    while decoder.can_decode() {
        decoder.decode_out(&mut instruction);
        buf.clear();
        formatter.format(&instruction, &mut buf);

        if regs.rip == instruction.ip() {
            write!(ret, "=> ")?;
        } else {
            write!(ret, "   ")?;
        }

        write!(ret, "0x{:x} ", instruction.ip())?;
        let start_index = (instruction.ip() - rip) as usize;
        let instr_bytes = &bytes[start_index..start_index + instruction.len()];
        for b in instr_bytes.iter() {
            write!(ret, "{:02x}", b)?;
        }
        if instr_bytes.len() < 7 {
            for _ in 0..7 - instr_bytes.len() {
                write!(ret, "  ")?;
            }
        }
        writeln!(ret, " {}", buf)?;
    }

    Ok(String::from_utf8_lossy(ret.as_slice()).to_string())
}

fn registers(subordinate: &Subordinate) -> Result<String> {
    let regs = subordinate.registers();
    let mut ret: Vec<u8> = Vec::new();

    writeln!(ret, "rip: 0x{:x}", regs.rip)?;
    writeln!(ret, "rsp: 0x{:x}", regs.rsp)?;
    writeln!(ret, "rbp: 0x{:x}", regs.rbp)?;
    writeln!(ret, "rax: 0x{:x}", regs.rax)?;
    writeln!(ret, "rbx: 0x{:x}", regs.rbx)?;
    writeln!(ret, "rcx: 0x{:x}", regs.rcx)?;
    writeln!(ret, "rdx: 0x{:x}", regs.rdx)?;
    writeln!(ret, "rdi: 0x{:x}", regs.rdi)?;
    writeln!(ret, "rsi: 0x{:x}", regs.rsi)?;

    Ok(String::from_utf8_lossy(ret.as_slice()).to_string())
}

fn stack(subordinate: &Subordinate) -> Result<String> {
    let stack = subordinate.stack();
    let mut ret: Vec<u8> = Vec::new();

    let rsp = subordinate.registers().rsp as usize;
    let wordlen = std::mem::size_of::<usize>();
    for (i, word) in stack.iter().enumerate() {
        writeln!(ret, "0x{:x}: 0x{:x}", rsp + wordlen * i, word)?;
    }

    Ok(String::from_utf8_lossy(ret.as_slice()).to_string())
}

fn execute_command(subordinate: &mut Subordinate, cmd: Vec<&str>) -> Result<()> {
    match cmd.as_slice() {
        ["s"] | ["step"] => subordinate.step()?,
        ["c"] | ["cont"] => subordinate.cont()?,
        ["b", addr] | ["break", addr] => set_breakpoint(subordinate, addr)?,
        other => return Err(format!("unknown command `{:?}`", other).into()),
    };

    Ok(())
}

fn set_breakpoint(subordinate: &mut Subordinate, addr: &str) -> Result<()> {
    if let Ok(addr) = usize::from_str_radix(addr, 16) {
        return subordinate.breakpoint(addr);
    }

    let fetch = {
        let symbols = subordinate.debug_info().symbols();
        symbols.get(addr).cloned()
    };

    if let Some(symbol) = fetch {
        return subordinate.breakpoint(symbol.low_pc as usize);
    }

    Err(format!(
        "couldn't set breakpoint on `{}`, not a known address or symbol",
        addr
    )
    .into())
}
