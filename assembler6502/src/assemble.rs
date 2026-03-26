/// Two-pass 6502 assembler.
/// Pass 1: assign addresses to labels.
/// Pass 2: emit bytes.
/// TODO: implement

pub struct Assembler;

impl Assembler {
    pub fn new() -> Self { Self }

    pub fn assemble(&self, _lines: &[String]) -> Vec<u8> {
        vec![]
    }
}
