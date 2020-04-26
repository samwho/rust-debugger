use crate::result::Result;
use iced_x86::{Decoder, DecoderOptions, Formatter, Instruction, NasmFormatter};
use std::io::prelude::*;

pub struct Disassembler {}

impl Disassembler {
    pub fn new() -> Self {
        Self {}
    }

    pub fn disassemble(&self, rip: u64, bytes: &[u8]) -> Result<String> {
        let mut decoder = Decoder::new(64, &bytes, DecoderOptions::NONE);
        decoder.set_ip(rip);

        let mut formatter = NasmFormatter::new();
        let mut ret: Vec<u8> = Vec::new();
        let mut buf = String::new();
        let mut instruction = Instruction::default();

        while decoder.can_decode() {
            decoder.decode_out(&mut instruction);
            if decoder.invalid_no_more_bytes() {
                break;
            }
            buf.clear();
            formatter.format(&instruction, &mut buf);

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
}
