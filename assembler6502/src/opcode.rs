#[derive(Debug,Clone,Copy,PartialEq,Eq,Hash)]
pub enum AddrMode{Imp,Acc,Imm,Zp,ZpX,ZpY,Abs,AbsX,AbsY,Ind,IndX,IndY,Rel}

#[derive(Debug,Clone,Copy)]
pub struct OpInfo{pub code:u8,pub mode:AddrMode}

use AddrMode::*;

// Basic opcode map keyed by (mnemonic, mode)
// Only official opcodes for now.
pub fn lookup(mnem:&str, mode:AddrMode)->Option<u8>{
    let m = mnem.to_ascii_uppercase();
    TABLE.iter().find(|o| o.0==m && o.2==mode).map(|o| o.1)
}

pub fn size_for(mode:AddrMode)->usize{
    match mode{
        AddrMode::Imp|AddrMode::Acc => 1,
        AddrMode::Imm|AddrMode::Zp|AddrMode::ZpX|AddrMode::ZpY|AddrMode::IndX|AddrMode::IndY|AddrMode::Rel => 2,
        AddrMode::Abs|AddrMode::AbsX|AddrMode::AbsY|AddrMode::Ind => 3,
    }
}

// (mnemonic, opcode, mode)
const TABLE:&[(&str,u8,AddrMode)]=&[
    // LDA
    ("LDA",0xA9,Imm),("LDA",0xA5,Zp),("LDA",0xB5,ZpX),("LDA",0xAD,Abs),("LDA",0xBD,AbsX),("LDA",0xB9,AbsY),("LDA",0xA1,IndX),("LDA",0xB1,IndY),
    ("LDX",0xA2,Imm),("LDX",0xA6,Zp),("LDX",0xB6,ZpY),("LDX",0xAE,Abs),("LDX",0xBE,AbsY),
    ("LDY",0xA0,Imm),("LDY",0xA4,Zp),("LDY",0xB4,ZpX),("LDY",0xAC,Abs),("LDY",0xBC,AbsX),
    ("STA",0x85,Zp),("STA",0x95,ZpX),("STA",0x8D,Abs),("STA",0x9D,AbsX),("STA",0x99,AbsY),("STA",0x81,IndX),("STA",0x91,IndY),
    ("STX",0x86,Zp),("STX",0x96,ZpY),("STX",0x8E,Abs),
    ("STY",0x84,Zp),("STY",0x94,ZpX),("STY",0x8C,Abs),
    ("ADC",0x69,Imm),("ADC",0x65,Zp),("ADC",0x75,ZpX),("ADC",0x6D,Abs),("ADC",0x7D,AbsX),("ADC",0x79,AbsY),("ADC",0x61,IndX),("ADC",0x71,IndY),
    ("SBC",0xE9,Imm),("SBC",0xE5,Zp),("SBC",0xF5,ZpX),("SBC",0xED,Abs),("SBC",0xFD,AbsX),("SBC",0xF9,AbsY),("SBC",0xE1,IndX),("SBC",0xF1,IndY),
    ("INC",0xE6,Zp),("INC",0xF6,ZpX),("INC",0xEE,Abs),("INC",0xFE,AbsX),
    ("INX",0xE8,Imp),("INY",0xC8,Imp),("DEX",0xCA,Imp),("DEY",0x88,Imp),
    ("DEC",0xC6,Zp),("DEC",0xD6,ZpX),("DEC",0xCE,Abs),("DEC",0xDE,AbsX),
    ("JMP",0x4C,Abs),("JMP",0x6C,Ind),("JSR",0x20,Abs),("RTS",0x60,Imp),("RTI",0x40,Imp),
    ("CLC",0x18,Imp),("SEC",0x38,Imp),("CLI",0x58,Imp),("SEI",0x78,Imp),("CLV",0xB8,Imp),("CLD",0xD8,Imp),("SED",0xF8,Imp),
    ("TAX",0xAA,Imp),("TXA",0x8A,Imp),("TAY",0xA8,Imp),("TYA",0x98,Imp),("TSX",0xBA,Imp),("TXS",0x9A,Imp),
    ("PHA",0x48,Imp),("PLA",0x68,Imp),("PHP",0x08,Imp),("PLP",0x28,Imp),
    ("AND",0x29,Imm),("AND",0x25,Zp),("AND",0x35,ZpX),("AND",0x2D,Abs),("AND",0x3D,AbsX),("AND",0x39,AbsY),("AND",0x21,IndX),("AND",0x31,IndY),
    ("ORA",0x09,Imm),("ORA",0x05,Zp),("ORA",0x15,ZpX),("ORA",0x0D,Abs),("ORA",0x1D,AbsX),("ORA",0x19,AbsY),("ORA",0x01,IndX),("ORA",0x11,IndY),
    ("EOR",0x49,Imm),("EOR",0x45,Zp),("EOR",0x55,ZpX),("EOR",0x4D,Abs),("EOR",0x5D,AbsX),("EOR",0x59,AbsY),("EOR",0x41,IndX),("EOR",0x51,IndY),
    ("CMP",0xC9,Imm),("CMP",0xC5,Zp),("CMP",0xD5,ZpX),("CMP",0xCD,Abs),("CMP",0xDD,AbsX),("CMP",0xD9,AbsY),("CMP",0xC1,IndX),("CMP",0xD1,IndY),
    ("CPX",0xE0,Imm),("CPX",0xE4,Zp),("CPX",0xEC,Abs), ("CPY",0xC0,Imm),("CPY",0xC4,Zp),("CPY",0xCC,Abs),
    ("BIT",0x24,Zp),("BIT",0x2C,Abs),
    ("ASL",0x0A,Acc),("ASL",0x06,Zp),("ASL",0x16,ZpX),("ASL",0x0E,Abs),("ASL",0x1E,AbsX),
    ("LSR",0x4A,Acc),("LSR",0x46,Zp),("LSR",0x56,ZpX),("LSR",0x4E,Abs),("LSR",0x5E,AbsX),
    ("ROL",0x2A,Acc),("ROL",0x26,Zp),("ROL",0x36,ZpX),("ROL",0x2E,Abs),("ROL",0x3E,AbsX),
    ("ROR",0x6A,Acc),("ROR",0x66,Zp),("ROR",0x76,ZpX),("ROR",0x6E,Abs),("ROR",0x7E,AbsX),
    ("BPL",0x10,Rel),("BMI",0x30,Rel),("BVC",0x50,Rel),("BVS",0x70,Rel),("BCC",0x90,Rel),("BCS",0xB0,Rel),("BNE",0xD0,Rel),("BEQ",0xF0,Rel),
    ("BRK",0x00,Imp),("NOP",0xEA,Imp),
];
