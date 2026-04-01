//! Expanded 6502 CPU core oriented toward Microsoft BASIC execution.
//! Cycle counts & some undocumented opcodes omitted for now

mod harness;
pub use harness::BasicHarness;

use tracing::trace;

// ── Status flag bit masks ──────────────────────────────────────────────────────
pub const FLAG_C: u8 = 0x01; // Carry
pub const FLAG_Z: u8 = 0x02; // Zero
pub const FLAG_I: u8 = 0x04; // IRQ Disable
pub const FLAG_D: u8 = 0x08; // Decimal
pub const FLAG_B: u8 = 0x10; // Break
pub const FLAG_U: u8 = 0x20; // Unused (always set when pushed)
pub const FLAG_V: u8 = 0x40; // Overflow
pub const FLAG_N: u8 = 0x80; // Negative

/// Named constants for every 6502 opcode implemented by this emulator.
/// Re-exported at crate root via `pub use opcodes::*`.
pub mod opcodes {
    // ── Load ──────────────────────────────────────────────────────────────────
    pub const LDA_IMM: u8 = 0xA9; // LDA #imm
    pub const LDA_ZP: u8 = 0xA5; // LDA zp
    pub const LDA_ZPX: u8 = 0xB5; // LDA zp,X
    pub const LDA_ABS: u8 = 0xAD; // LDA abs
    pub const LDA_ABX: u8 = 0xBD; // LDA abs,X
    pub const LDA_ABY: u8 = 0xB9; // LDA abs,Y
    pub const LDA_INX: u8 = 0xA1; // LDA (zp,X)
    pub const LDA_INY: u8 = 0xB1; // LDA (zp),Y

    pub const LDX_IMM: u8 = 0xA2; // LDX #imm
    pub const LDX_ZP: u8 = 0xA6; // LDX zp
    pub const LDX_ZPY: u8 = 0xB6; // LDX zp,Y
    pub const LDX_ABS: u8 = 0xAE; // LDX abs
    pub const LDX_ABY: u8 = 0xBE; // LDX abs,Y

    pub const LDY_IMM: u8 = 0xA0; // LDY #imm
    pub const LDY_ZP: u8 = 0xA4; // LDY zp
    pub const LDY_ZPX: u8 = 0xB4; // LDY zp,X
    pub const LDY_ABS: u8 = 0xAC; // LDY abs
    pub const LDY_ABX: u8 = 0xBC; // LDY abs,X

    // ── Store ─────────────────────────────────────────────────────────────────
    pub const STA_ZP: u8 = 0x85; // STA zp
    pub const STA_ZPX: u8 = 0x95; // STA zp,X
    pub const STA_ABS: u8 = 0x8D; // STA abs
    pub const STA_ABX: u8 = 0x9D; // STA abs,X
    pub const STA_ABY: u8 = 0x99; // STA abs,Y
    pub const STA_INX: u8 = 0x81; // STA (zp,X)
    pub const STA_INY: u8 = 0x91; // STA (zp),Y

    pub const STX_ZP: u8 = 0x86; // STX zp
    pub const STX_ZPY: u8 = 0x96; // STX zp,Y
    pub const STX_ABS: u8 = 0x8E; // STX abs

    pub const STY_ZP: u8 = 0x84; // STY zp
    pub const STY_ZPX: u8 = 0x94; // STY zp,X
    pub const STY_ABS: u8 = 0x8C; // STY abs

    // ── Transfers ─────────────────────────────────────────────────────────────
    pub const TAX: u8 = 0xAA;
    pub const TXA: u8 = 0x8A;
    pub const TAY: u8 = 0xA8;
    pub const TYA: u8 = 0x98;
    pub const TSX: u8 = 0xBA;
    pub const TXS: u8 = 0x9A;

    // ── Increment / Decrement ─────────────────────────────────────────────────
    pub const INX: u8 = 0xE8;
    pub const INY: u8 = 0xC8;
    pub const DEX: u8 = 0xCA;
    pub const DEY: u8 = 0x88;

    pub const INC_ZP: u8 = 0xE6; // INC zp
    pub const INC_ZPX: u8 = 0xF6; // INC zp,X
    pub const INC_ABS: u8 = 0xEE; // INC abs
    pub const INC_ABX: u8 = 0xFE; // INC abs,X

    pub const DEC_ZP: u8 = 0xC6; // DEC zp
    pub const DEC_ZPX: u8 = 0xD6; // DEC zp,X
    pub const DEC_ABS: u8 = 0xCE; // DEC abs
    pub const DEC_ABX: u8 = 0xDE; // DEC abs,X

    // ── Arithmetic ────────────────────────────────────────────────────────────
    pub const ADC_IMM: u8 = 0x69;
    pub const ADC_ZP: u8 = 0x65;
    pub const ADC_ZPX: u8 = 0x75;
    pub const ADC_ABS: u8 = 0x6D;
    pub const ADC_ABX: u8 = 0x7D;
    pub const ADC_ABY: u8 = 0x79;
    pub const ADC_INX: u8 = 0x61;
    pub const ADC_INY: u8 = 0x71;

    pub const SBC_IMM: u8 = 0xE9;
    pub const SBC_IMM_ALT: u8 = 0xEB; // undocumented alias
    pub const SBC_ZP: u8 = 0xE5;
    pub const SBC_ZPX: u8 = 0xF5;
    pub const SBC_ABS: u8 = 0xED;
    pub const SBC_ABX: u8 = 0xFD;
    pub const SBC_ABY: u8 = 0xF9;
    pub const SBC_INX: u8 = 0xE1;
    pub const SBC_INY: u8 = 0xF1;

    // ── Logic ─────────────────────────────────────────────────────────────────
    pub const AND_IMM: u8 = 0x29;
    pub const AND_ZP: u8 = 0x25;
    pub const AND_ZPX: u8 = 0x35;
    pub const AND_ABS: u8 = 0x2D;
    pub const AND_ABX: u8 = 0x3D;
    pub const AND_ABY: u8 = 0x39;
    pub const AND_INX: u8 = 0x21;
    pub const AND_INY: u8 = 0x31;

    pub const ORA_IMM: u8 = 0x09;
    pub const ORA_ZP: u8 = 0x05;
    pub const ORA_ZPX: u8 = 0x15;
    pub const ORA_ABS: u8 = 0x0D;
    pub const ORA_ABX: u8 = 0x1D;
    pub const ORA_ABY: u8 = 0x19;
    pub const ORA_INX: u8 = 0x01;
    pub const ORA_INY: u8 = 0x11;

    pub const EOR_IMM: u8 = 0x49;
    pub const EOR_ZP: u8 = 0x45;
    pub const EOR_ZPX: u8 = 0x55;
    pub const EOR_ABS: u8 = 0x4D;
    pub const EOR_ABX: u8 = 0x5D;
    pub const EOR_ABY: u8 = 0x59;
    pub const EOR_INX: u8 = 0x41;
    pub const EOR_INY: u8 = 0x51;

    pub const BIT_ZP: u8 = 0x24;
    pub const BIT_ABS: u8 = 0x2C;

    // ── Compare ───────────────────────────────────────────────────────────────
    pub const CMP_IMM: u8 = 0xC9;
    pub const CMP_ZP: u8 = 0xC5;
    pub const CMP_ZPX: u8 = 0xD5;
    pub const CMP_ABS: u8 = 0xCD;
    pub const CMP_ABX: u8 = 0xDD;
    pub const CMP_ABY: u8 = 0xD9;
    pub const CMP_INX: u8 = 0xC1;
    pub const CMP_INY: u8 = 0xD1;

    pub const CPX_IMM: u8 = 0xE0;
    pub const CPX_ZP: u8 = 0xE4;
    pub const CPX_ABS: u8 = 0xEC;

    pub const CPY_IMM: u8 = 0xC0;
    pub const CPY_ZP: u8 = 0xC4;
    pub const CPY_ABS: u8 = 0xCC;

    // ── Shifts & Rotates ──────────────────────────────────────────────────────
    pub const ASL_ACC: u8 = 0x0A; // ASL A (accumulator)
    pub const ASL_ZP: u8 = 0x06;
    pub const ASL_ZPX: u8 = 0x16;
    pub const ASL_ABS: u8 = 0x0E;
    pub const ASL_ABX: u8 = 0x1E;

    pub const LSR_ACC: u8 = 0x4A; // LSR A
    pub const LSR_ZP: u8 = 0x46;
    pub const LSR_ZPX: u8 = 0x56;
    pub const LSR_ABS: u8 = 0x4E;
    pub const LSR_ABX: u8 = 0x5E;

    pub const ROL_ACC: u8 = 0x2A; // ROL A
    pub const ROL_ZP: u8 = 0x26;
    pub const ROL_ZPX: u8 = 0x36;
    pub const ROL_ABS: u8 = 0x2E;
    pub const ROL_ABX: u8 = 0x3E;

    pub const ROR_ACC: u8 = 0x6A; // ROR A
    pub const ROR_ZP: u8 = 0x66;
    pub const ROR_ZPX: u8 = 0x76;
    pub const ROR_ABS: u8 = 0x6E;
    pub const ROR_ABX: u8 = 0x7E;

    // ── Branches ──────────────────────────────────────────────────────────────
    pub const BPL: u8 = 0x10; // Branch if Plus (N=0)
    pub const BMI: u8 = 0x30; // Branch if Minus (N=1)
    pub const BVC: u8 = 0x50; // Branch if oVerflow Clear
    pub const BVS: u8 = 0x70; // Branch if oVerflow Set
    pub const BCC: u8 = 0x90; // Branch if Carry Clear
    pub const BCS: u8 = 0xB0; // Branch if Carry Set
    pub const BNE: u8 = 0xD0; // Branch if Not Equal (Z=0)
    pub const BEQ: u8 = 0xF0; // Branch if EQual (Z=1)

    // ── Jumps & Calls ─────────────────────────────────────────────────────────
    pub const JMP_ABS: u8 = 0x4C; // JMP abs
    pub const JMP_IND: u8 = 0x6C; // JMP (abs)
    pub const JSR: u8 = 0x20; // Jump to SubRoutine
    pub const RTS: u8 = 0x60; // ReTurn from Subroutine
    pub const RTI: u8 = 0x40; // ReTurn from Interrupt

    // ── Stack ─────────────────────────────────────────────────────────────────
    pub const PHA: u8 = 0x48; // PusH Accumulator
    pub const PLA: u8 = 0x68; // PuLl Accumulator
    pub const PHP: u8 = 0x08; // PusH Processor status
    pub const PLP: u8 = 0x28; // PuLl Processor status

    // ── Flag Control ──────────────────────────────────────────────────────────
    pub const CLC: u8 = 0x18; // CLear Carry
    pub const SEC: u8 = 0x38; // SEt Carry
    pub const CLI: u8 = 0x58; // CLear Interrupt disable
    pub const SEI: u8 = 0x78; // SEt Interrupt disable
    pub const CLV: u8 = 0xB8; // CLear oVerflow
    pub const CLD: u8 = 0xD8; // CLear Decimal
    pub const SED: u8 = 0xF8; // SEt Decimal

    // ── Control ───────────────────────────────────────────────────────────────
    pub const BRK: u8 = 0x00; // BReaK / software interrupt
    pub const NOP: u8 = 0xEA; // No OPeration
}

pub use opcodes::*;

/// Return the mnemonic string for a given opcode byte (used in trace output).
pub fn opcode_name(op: u8) -> &'static str {
    use opcodes::*;
    match op {
        LDA_IMM => "LDA #",
        LDA_ZP => "LDA zp",
        LDA_ZPX => "LDA zp,X",
        LDA_ABS => "LDA abs",
        LDA_ABX => "LDA abs,X",
        LDA_ABY => "LDA abs,Y",
        LDA_INX => "LDA (zp,X)",
        LDA_INY => "LDA (zp),Y",
        LDX_IMM => "LDX #",
        LDX_ZP => "LDX zp",
        LDX_ZPY => "LDX zp,Y",
        LDX_ABS => "LDX abs",
        LDX_ABY => "LDX abs,Y",
        LDY_IMM => "LDY #",
        LDY_ZP => "LDY zp",
        LDY_ZPX => "LDY zp,X",
        LDY_ABS => "LDY abs",
        LDY_ABX => "LDY abs,X",
        STA_ZP => "STA zp",
        STA_ZPX => "STA zp,X",
        STA_ABS => "STA abs",
        STA_ABX => "STA abs,X",
        STA_ABY => "STA abs,Y",
        STA_INX => "STA (zp,X)",
        STA_INY => "STA (zp),Y",
        STX_ZP => "STX zp",
        STX_ZPY => "STX zp,Y",
        STX_ABS => "STX abs",
        STY_ZP => "STY zp",
        STY_ZPX => "STY zp,X",
        STY_ABS => "STY abs",
        TAX => "TAX",
        TXA => "TXA",
        TAY => "TAY",
        TYA => "TYA",
        TSX => "TSX",
        TXS => "TXS",
        INX => "INX",
        INY => "INY",
        DEX => "DEX",
        DEY => "DEY",
        INC_ZP => "INC zp",
        INC_ZPX => "INC zp,X",
        INC_ABS => "INC abs",
        INC_ABX => "INC abs,X",
        DEC_ZP => "DEC zp",
        DEC_ZPX => "DEC zp,X",
        DEC_ABS => "DEC abs",
        DEC_ABX => "DEC abs,X",
        ADC_IMM => "ADC #",
        ADC_ZP => "ADC zp",
        ADC_ZPX => "ADC zp,X",
        ADC_ABS => "ADC abs",
        ADC_ABX => "ADC abs,X",
        ADC_ABY => "ADC abs,Y",
        ADC_INX => "ADC (zp,X)",
        ADC_INY => "ADC (zp),Y",
        SBC_IMM | SBC_IMM_ALT => "SBC #",
        SBC_ZP => "SBC zp",
        SBC_ZPX => "SBC zp,X",
        SBC_ABS => "SBC abs",
        SBC_ABX => "SBC abs,X",
        SBC_ABY => "SBC abs,Y",
        SBC_INX => "SBC (zp,X)",
        SBC_INY => "SBC (zp),Y",
        AND_IMM => "AND #",
        AND_ZP => "AND zp",
        AND_ZPX => "AND zp,X",
        AND_ABS => "AND abs",
        AND_ABX => "AND abs,X",
        AND_ABY => "AND abs,Y",
        AND_INX => "AND (zp,X)",
        AND_INY => "AND (zp),Y",
        ORA_IMM => "ORA #",
        ORA_ZP => "ORA zp",
        ORA_ZPX => "ORA zp,X",
        ORA_ABS => "ORA abs",
        ORA_ABX => "ORA abs,X",
        ORA_ABY => "ORA abs,Y",
        ORA_INX => "ORA (zp,X)",
        ORA_INY => "ORA (zp),Y",
        EOR_IMM => "EOR #",
        EOR_ZP => "EOR zp",
        EOR_ZPX => "EOR zp,X",
        EOR_ABS => "EOR abs",
        EOR_ABX => "EOR abs,X",
        EOR_ABY => "EOR abs,Y",
        EOR_INX => "EOR (zp,X)",
        EOR_INY => "EOR (zp),Y",
        BIT_ZP => "BIT zp",
        BIT_ABS => "BIT abs",
        CMP_IMM => "CMP #",
        CMP_ZP => "CMP zp",
        CMP_ZPX => "CMP zp,X",
        CMP_ABS => "CMP abs",
        CMP_ABX => "CMP abs,X",
        CMP_ABY => "CMP abs,Y",
        CMP_INX => "CMP (zp,X)",
        CMP_INY => "CMP (zp),Y",
        CPX_IMM => "CPX #",
        CPX_ZP => "CPX zp",
        CPX_ABS => "CPX abs",
        CPY_IMM => "CPY #",
        CPY_ZP => "CPY zp",
        CPY_ABS => "CPY abs",
        ASL_ACC => "ASL A",
        ASL_ZP => "ASL zp",
        ASL_ZPX => "ASL zp,X",
        ASL_ABS => "ASL abs",
        ASL_ABX => "ASL abs,X",
        LSR_ACC => "LSR A",
        LSR_ZP => "LSR zp",
        LSR_ZPX => "LSR zp,X",
        LSR_ABS => "LSR abs",
        LSR_ABX => "LSR abs,X",
        ROL_ACC => "ROL A",
        ROL_ZP => "ROL zp",
        ROL_ZPX => "ROL zp,X",
        ROL_ABS => "ROL abs",
        ROL_ABX => "ROL abs,X",
        ROR_ACC => "ROR A",
        ROR_ZP => "ROR zp",
        ROR_ZPX => "ROR zp,X",
        ROR_ABS => "ROR abs",
        ROR_ABX => "ROR abs,X",
        BPL => "BPL",
        BMI => "BMI",
        BVC => "BVC",
        BVS => "BVS",
        BCC => "BCC",
        BCS => "BCS",
        BNE => "BNE",
        BEQ => "BEQ",
        JMP_ABS => "JMP abs",
        JMP_IND => "JMP (abs)",
        JSR => "JSR",
        RTS => "RTS",
        RTI => "RTI",
        PHA => "PHA",
        PLA => "PLA",
        PHP => "PHP",
        PLP => "PLP",
        CLC => "CLC",
        SEC => "SEC",
        CLI => "CLI",
        SEI => "SEI",
        CLV => "CLV",
        CLD => "CLD",
        SED => "SED",
        BRK => "BRK",
        NOP => "NOP",
        _ => "???",
    }
}

pub struct Cpu {
    pub a: u8,
    pub x: u8,
    pub y: u8,
    pub sp: u8,
    pub p: u8,
    pub pc: u16,
    pub mem: [u8; 65536],
    pub cycles: u64,
    pub halted: bool,
    irq_pending: bool,
    nmi_pending: bool,
    trap_brk: bool,
    read_hooks: Vec<(u16, Box<dyn Fn(&Cpu, u16) -> u8>)>,
    write_hooks: Vec<(u16, Box<dyn Fn(&mut Cpu, u16, u8)>)>,
    /// PC-trap hooks: when PC == addr just before fetch, call the closure instead of executing.
    /// The closure receives &mut Cpu and should perform an RTS behaviour (pop stack, set PC).
    exec_hooks: Vec<(u16, Box<dyn FnMut(&mut Cpu)>)>,
}

impl Default for Cpu {
    fn default() -> Self {
        Self::new()
    }
}

impl Cpu {
    pub fn new() -> Self {
        Self {
            a: 0,
            x: 0,
            y: 0,
            sp: 0xFD,
            p: FLAG_U | FLAG_I,
            pc: 0,
            mem: [0; 65536],
            cycles: 0,
            halted: false,
            irq_pending: false,
            nmi_pending: false,
            trap_brk: true,
            read_hooks: Vec::new(),
            write_hooks: Vec::new(),
            exec_hooks: Vec::new(),
        }
    }
    pub fn load(&mut self, addr: u16, bytes: &[u8]) {
        self.mem[addr as usize..addr as usize + bytes.len()].copy_from_slice(bytes);
    }
    pub fn reset(&mut self) {
        self.sp = 0xFD;
        let lo = self.mem[0xFFFC] as u16;
        let hi = self.mem[0xFFFD] as u16;
        self.pc = lo | (hi << 8);
        self.halted = false;
    }

    // Public run helper (cycle bounded)
    pub fn run_cycles(&mut self, max: u64) {
        while self.cycles < max && !self.halted {
            self.step();
        }
    }
    // Convenience: run until BRK (halt) or cycle budget exceeded
    pub fn run_until_brk(&mut self, max_cycles: u64) -> Result<(), &'static str> {
        let start = self.cycles;
        while !self.halted && (self.cycles - start) < max_cycles {
            self.step();
        }
        if self.halted {
            Ok(())
        } else {
            Err("cycle budget exceeded without BRK")
        }
    }
    // Generic predicate runner (returns true when predicate returns true) with cycle cap
    pub fn run_until<F: Fn(&Cpu) -> bool>(
        &mut self,
        max_cycles: u64,
        pred: F,
    ) -> Result<(), &'static str> {
        let start = self.cycles;
        while (self.cycles - start) < max_cycles {
            if pred(self) {
                return Ok(());
            }
            if self.halted {
                return Ok(());
            }
            self.step();
        }
        Err("cycle budget exceeded")
    }
    // Hook registration
    pub fn hook_read<F: 'static + Fn(&Cpu, u16) -> u8>(&mut self, addr: u16, f: F) {
        self.read_hooks.push((addr, Box::new(f)));
    }
    pub fn hook_write<F: 'static + Fn(&mut Cpu, u16, u8)>(&mut self, addr: u16, f: F) {
        self.write_hooks.push((addr, Box::new(f)));
    }
    /// Register a hook that fires when PC reaches `addr` (before the instruction is fetched).
    /// The closure should simulate the routine (e.g. consume A, then do RTS via `cpu.trap_rts()`).
    pub fn hook_exec<F: 'static + FnMut(&mut Cpu)>(&mut self, addr: u16, f: F) {
        self.exec_hooks.push((addr, Box::new(f)));
    }
    /// Simulate RTS: pop 16-bit return address from stack, add 1, set PC.
    pub fn trap_rts(&mut self) {
        let lo = self.pop() as u16;
        let hi = self.pop() as u16;
        self.pc = ((hi << 8) | lo).wrapping_add(1);
    }

    // ── Core step ─────────────────────────────────────────────────────────────
    pub fn step(&mut self) {
        if self.halted {
            return;
        }
        // Exec-trap: check PC against registered hooks before fetching the opcode.
        // We iterate by index to satisfy the borrow checker (hooks need &mut Cpu).
        for idx in 0..self.exec_hooks.len() {
            if self.exec_hooks[idx].0 == self.pc {
                trace!(
                    target: "emu6502::hooks",
                    "EXEC HOOK at ${:04X}", self.pc
                );
                // Swap out the closure, call it, swap back.
                let mut hook =
                    std::mem::replace(&mut self.exec_hooks[idx].1, Box::new(|_: &mut Cpu| {}));
                hook(self);
                self.exec_hooks[idx].1 = hook;
                self.cycles += 6; // approximate JSR cost
                return;
            }
        }
        // Interrupt poll (NMI first)
        if self.nmi_pending {
            self.nmi_pending = false;
            self.interrupt(vec_nmi());
        } else if self.irq_pending && (self.p & FLAG_I) == 0 {
            self.irq_pending = false;
            self.interrupt(vec_irq());
        }
        if self.halted {
            return; // interrupt may set halted (if vector 0 and trap)
        }

        let instr_pc = self.pc;
        let op = self.fetch8();

        trace!(
            target: "emu6502::cpu",
            pc   = %format!("${:04X}", instr_pc),
            op   = %format!("{:02X}", op),
            insn = opcode_name(op),
            a    = %format!("${:02X}", self.a),
            x    = %format!("${:02X}", self.x),
            y    = %format!("${:02X}", self.y),
            sp   = %format!("${:02X}", self.sp),
            sr   = %format!("${:02X}", self.p),
        );

        use opcodes::*;
        let mut extra_cycles = 0u8;
        match op {
            // ── Load / Store ──────────────────────────────────────────────────
            LDA_IMM => {
                let v = self.imm();
                self.a = v;
                self.nz(v);
            }
            LDA_ZP => {
                let a = self.zp();
                let v = self.read(a);
                self.a = v;
                self.nz(v);
            }
            LDA_ZPX => {
                let a = self.zpx();
                let v = self.read(a);
                self.a = v;
                self.nz(v);
            }
            LDA_ABS => {
                let a = self.abs();
                let v = self.read(a);
                self.a = v;
                self.nz(v);
            }
            LDA_ABX => {
                let (a, pc) = self.absx_pc();
                if pc {
                    extra_cycles += 1;
                }
                let v = self.read(a);
                self.a = v;
                self.nz(v);
            }
            LDA_ABY => {
                let (a, pc) = self.absy_pc();
                if pc {
                    extra_cycles += 1;
                }
                let v = self.read(a);
                self.a = v;
                self.nz(v);
            }
            LDA_INX => {
                let a = self.indx();
                let v = self.read(a);
                self.a = v;
                self.nz(v);
            }
            LDA_INY => {
                let a = self.indy();
                let v = self.read(a);
                self.a = v;
                self.nz(v);
            }

            LDX_IMM => {
                let v = self.imm();
                self.x = v;
                self.nz(v);
            }
            LDX_ZP => {
                let a = self.zp();
                let v = self.read(a);
                self.x = v;
                self.nz(v);
            }
            LDX_ZPY => {
                let a = self.zpy();
                let v = self.read(a);
                self.x = v;
                self.nz(v);
            }
            LDX_ABS => {
                let a = self.abs();
                let v = self.read(a);
                self.x = v;
                self.nz(v);
            }
            LDX_ABY => {
                let (a, pc) = self.absy_pc();
                if pc {
                    extra_cycles += 1;
                }
                let v = self.read(a);
                self.x = v;
                self.nz(v);
            }

            LDY_IMM => {
                let v = self.imm();
                self.y = v;
                self.nz(v);
            }
            LDY_ZP => {
                let a = self.zp();
                let v = self.read(a);
                self.y = v;
                self.nz(v);
            }
            LDY_ZPX => {
                let a = self.zpx();
                let v = self.read(a);
                self.y = v;
                self.nz(v);
            }
            LDY_ABS => {
                let a = self.abs();
                let v = self.read(a);
                self.y = v;
                self.nz(v);
            }
            LDY_ABX => {
                let (a, pc) = self.absx_pc();
                if pc {
                    extra_cycles += 1;
                }
                let v = self.read(a);
                self.y = v;
                self.nz(v);
            }

            STA_ZP => {
                let a = self.zp();
                self.write(a, self.a);
            }
            STA_ZPX => {
                let a = self.zpx();
                self.write(a, self.a);
            }
            STA_ABS => {
                let a = self.abs();
                self.write(a, self.a);
            }
            STA_ABX => {
                let a = self.absx();
                self.write(a, self.a);
            }
            STA_ABY => {
                let a = self.absy();
                self.write(a, self.a);
            }
            STA_INX => {
                let a = self.indx();
                self.write(a, self.a);
            }
            STA_INY => {
                let a = self.indy();
                self.write(a, self.a);
            }

            STX_ZP => {
                let a = self.zp();
                self.write(a, self.x);
            }
            STX_ZPY => {
                let a = self.zpy();
                self.write(a, self.x);
            }
            STX_ABS => {
                let a = self.abs();
                self.write(a, self.x);
            }

            STY_ZP => {
                let a = self.zp();
                self.write(a, self.y);
            }
            STY_ZPX => {
                let a = self.zpx();
                self.write(a, self.y);
            }
            STY_ABS => {
                let a = self.abs();
                self.write(a, self.y);
            }

            // ── Transfers ─────────────────────────────────────────────────────
            TAX => {
                self.x = self.a;
                self.nz(self.x);
            }
            TXA => {
                self.a = self.x;
                self.nz(self.a);
            }
            TAY => {
                self.y = self.a;
                self.nz(self.y);
            }
            TYA => {
                self.a = self.y;
                self.nz(self.a);
            }
            TSX => {
                self.x = self.sp;
                self.nz(self.x);
            }
            TXS => {
                self.sp = self.x;
            }

            // ── Increments / Decrements ───────────────────────────────────────
            INX => {
                self.x = self.x.wrapping_add(1);
                self.nz(self.x);
            }
            INY => {
                self.y = self.y.wrapping_add(1);
                self.nz(self.y);
            }
            DEX => {
                self.x = self.x.wrapping_sub(1);
                self.nz(self.x);
            }
            DEY => {
                self.y = self.y.wrapping_sub(1);
                self.nz(self.y);
            }

            INC_ZP => {
                let a = self.zp();
                let v = self.read(a).wrapping_add(1);
                self.write(a, v);
                self.nz(v);
            }
            INC_ZPX => {
                let a = self.zpx();
                let v = self.read(a).wrapping_add(1);
                self.write(a, v);
                self.nz(v);
            }
            INC_ABS => {
                let a = self.abs();
                let v = self.read(a).wrapping_add(1);
                self.write(a, v);
                self.nz(v);
            }
            INC_ABX => {
                let a = self.absx();
                let v = self.read(a).wrapping_add(1);
                self.write(a, v);
                self.nz(v);
            }

            DEC_ZP => {
                let a = self.zp();
                let v = self.read(a).wrapping_sub(1);
                self.write(a, v);
                self.nz(v);
            }
            DEC_ZPX => {
                let a = self.zpx();
                let v = self.read(a).wrapping_sub(1);
                self.write(a, v);
                self.nz(v);
            }
            DEC_ABS => {
                let a = self.abs();
                let v = self.read(a).wrapping_sub(1);
                self.write(a, v);
                self.nz(v);
            }
            DEC_ABX => {
                let a = self.absx();
                let v = self.read(a).wrapping_sub(1);
                self.write(a, v);
                self.nz(v);
            }

            // ── Arithmetic ────────────────────────────────────────────────────
            ADC_IMM => {
                let v = self.imm();
                self.adc(v);
            }
            ADC_ZP => {
                let a = self.zp();
                let v = self.read(a);
                self.adc(v);
            }
            ADC_ZPX => {
                let a = self.zpx();
                let v = self.read(a);
                self.adc(v);
            }
            ADC_ABS => {
                let a = self.abs();
                let v = self.read(a);
                self.adc(v);
            }
            ADC_ABX => {
                let (a, pc) = self.absx_pc();
                if pc {
                    extra_cycles += 1;
                }
                let v = self.read(a);
                self.adc(v);
            }
            ADC_ABY => {
                let (a, pc) = self.absy_pc();
                if pc {
                    extra_cycles += 1;
                }
                let v = self.read(a);
                self.adc(v);
            }
            ADC_INX => {
                let a = self.indx();
                let v = self.read(a);
                self.adc(v);
            }
            ADC_INY => {
                let (a, pc) = self.indy_pc();
                if pc {
                    extra_cycles += 1;
                }
                let v = self.read(a);
                self.adc(v);
            }

            SBC_IMM | SBC_IMM_ALT => {
                let v = self.imm();
                self.sbc(v);
            }
            SBC_ZP => {
                let a = self.zp();
                let v = self.read(a);
                self.sbc(v);
            }
            SBC_ZPX => {
                let a = self.zpx();
                let v = self.read(a);
                self.sbc(v);
            }
            SBC_ABS => {
                let a = self.abs();
                let v = self.read(a);
                self.sbc(v);
            }
            SBC_ABX => {
                let (a, pc) = self.absx_pc();
                if pc {
                    extra_cycles += 1;
                }
                let v = self.read(a);
                self.sbc(v);
            }
            SBC_ABY => {
                let (a, pc) = self.absy_pc();
                if pc {
                    extra_cycles += 1;
                }
                let v = self.read(a);
                self.sbc(v);
            }
            SBC_INX => {
                let a = self.indx();
                let v = self.read(a);
                self.sbc(v);
            }
            SBC_INY => {
                let (a, pc) = self.indy_pc();
                if pc {
                    extra_cycles += 1;
                }
                let v = self.read(a);
                self.sbc(v);
            }

            // ── Logic ─────────────────────────────────────────────────────────
            AND_IMM => {
                let v = self.imm();
                self.a &= v;
                self.nz(self.a);
            }
            AND_ZP => {
                let a = self.zp();
                let v = self.read(a);
                self.a &= v;
                self.nz(self.a);
            }
            AND_ZPX => {
                let a = self.zpx();
                let v = self.read(a);
                self.a &= v;
                self.nz(self.a);
            }
            AND_ABS => {
                let a = self.abs();
                let v = self.read(a);
                self.a &= v;
                self.nz(self.a);
            }
            AND_ABX => {
                let (a, pc) = self.absx_pc();
                if pc {
                    extra_cycles += 1;
                }
                let v = self.read(a);
                self.a &= v;
                self.nz(self.a);
            }
            AND_ABY => {
                let (a, pc) = self.absy_pc();
                if pc {
                    extra_cycles += 1;
                }
                let v = self.read(a);
                self.a &= v;
                self.nz(self.a);
            }
            AND_INX => {
                let a = self.indx();
                let v = self.read(a);
                self.a &= v;
                self.nz(self.a);
            }
            AND_INY => {
                let (a, pc) = self.indy_pc();
                if pc {
                    extra_cycles += 1;
                }
                let v = self.read(a);
                self.a &= v;
                self.nz(self.a);
            }

            ORA_IMM => {
                let v = self.imm();
                self.a |= v;
                self.nz(self.a);
            }
            ORA_ZP => {
                let a = self.zp();
                let v = self.read(a);
                self.a |= v;
                self.nz(self.a);
            }
            ORA_ZPX => {
                let a = self.zpx();
                let v = self.read(a);
                self.a |= v;
                self.nz(self.a);
            }
            ORA_ABS => {
                let a = self.abs();
                let v = self.read(a);
                self.a |= v;
                self.nz(self.a);
            }
            ORA_ABX => {
                let (a, pc) = self.absx_pc();
                if pc {
                    extra_cycles += 1;
                }
                let v = self.read(a);
                self.a |= v;
                self.nz(self.a);
            }
            ORA_ABY => {
                let (a, pc) = self.absy_pc();
                if pc {
                    extra_cycles += 1;
                }
                let v = self.read(a);
                self.a |= v;
                self.nz(self.a);
            }
            ORA_INX => {
                let a = self.indx();
                let v = self.read(a);
                self.a |= v;
                self.nz(self.a);
            }
            ORA_INY => {
                let (a, pc) = self.indy_pc();
                if pc {
                    extra_cycles += 1;
                }
                let v = self.read(a);
                self.a |= v;
                self.nz(self.a);
            }

            EOR_IMM => {
                let v = self.imm();
                self.a ^= v;
                self.nz(self.a);
            }
            EOR_ZP => {
                let a = self.zp();
                let v = self.read(a);
                self.a ^= v;
                self.nz(self.a);
            }
            EOR_ZPX => {
                let a = self.zpx();
                let v = self.read(a);
                self.a ^= v;
                self.nz(self.a);
            }
            EOR_ABS => {
                let a = self.abs();
                let v = self.read(a);
                self.a ^= v;
                self.nz(self.a);
            }
            EOR_ABX => {
                let (a, pc) = self.absx_pc();
                if pc {
                    extra_cycles += 1;
                }
                let v = self.read(a);
                self.a ^= v;
                self.nz(self.a);
            }
            EOR_ABY => {
                let (a, pc) = self.absy_pc();
                if pc {
                    extra_cycles += 1;
                }
                let v = self.read(a);
                self.a ^= v;
                self.nz(self.a);
            }
            EOR_INX => {
                let a = self.indx();
                let v = self.read(a);
                self.a ^= v;
                self.nz(self.a);
            }
            EOR_INY => {
                let (a, pc) = self.indy_pc();
                if pc {
                    extra_cycles += 1;
                }
                let v = self.read(a);
                self.a ^= v;
                self.nz(self.a);
            }

            BIT_ZP => {
                let a = self.zp();
                let v = self.read(a);
                self.bit(v);
            }
            BIT_ABS => {
                let a = self.abs();
                let v = self.read(a);
                self.bit(v);
            }

            // ── Compare ───────────────────────────────────────────────────────
            CMP_IMM => {
                let v = self.imm();
                self.cmp(self.a, v);
            }
            CMP_ZP => {
                let a = self.zp();
                let v = self.read(a);
                self.cmp(self.a, v);
            }
            CMP_ZPX => {
                let a = self.zpx();
                let v = self.read(a);
                self.cmp(self.a, v);
            }
            CMP_ABS => {
                let a = self.abs();
                let v = self.read(a);
                self.cmp(self.a, v);
            }
            CMP_ABX => {
                let (a, pc) = self.absx_pc();
                if pc {
                    extra_cycles += 1;
                }
                let v = self.read(a);
                self.cmp(self.a, v);
            }
            CMP_ABY => {
                let (a, pc) = self.absy_pc();
                if pc {
                    extra_cycles += 1;
                }
                let v = self.read(a);
                self.cmp(self.a, v);
            }
            CMP_INX => {
                let a = self.indx();
                let v = self.read(a);
                self.cmp(self.a, v);
            }
            CMP_INY => {
                let (a, pc) = self.indy_pc();
                if pc {
                    extra_cycles += 1;
                }
                let v = self.read(a);
                self.cmp(self.a, v);
            }

            CPX_IMM => {
                let v = self.imm();
                self.cmp(self.x, v);
            }
            CPX_ZP => {
                let a = self.zp();
                let v = self.read(a);
                self.cmp(self.x, v);
            }
            CPX_ABS => {
                let a = self.abs();
                let v = self.read(a);
                self.cmp(self.x, v);
            }

            CPY_IMM => {
                let v = self.imm();
                self.cmp(self.y, v);
            }
            CPY_ZP => {
                let a = self.zp();
                let v = self.read(a);
                self.cmp(self.y, v);
            }
            CPY_ABS => {
                let a = self.abs();
                let v = self.read(a);
                self.cmp(self.y, v);
            }

            // ── Shifts & Rotates ──────────────────────────────────────────────
            ASL_ACC => {
                let c = (self.a >> 7) & 1;
                self.a <<= 1;
                self.set_c(c != 0);
                self.nz(self.a);
            }
            ASL_ZP => {
                let a = self.zp();
                let mut v = self.read(a);
                let c = (v >> 7) & 1;
                v <<= 1;
                self.write(a, v);
                self.set_c(c != 0);
                self.nz(v);
            }
            ASL_ZPX => {
                let a = self.zpx();
                let mut v = self.read(a);
                let c = (v >> 7) & 1;
                v <<= 1;
                self.write(a, v);
                self.set_c(c != 0);
                self.nz(v);
            }
            ASL_ABS => {
                let a = self.abs();
                let mut v = self.read(a);
                let c = (v >> 7) & 1;
                v <<= 1;
                self.write(a, v);
                self.set_c(c != 0);
                self.nz(v);
            }
            ASL_ABX => {
                let a = self.absx();
                let mut v = self.read(a);
                let c = (v >> 7) & 1;
                v <<= 1;
                self.write(a, v);
                self.set_c(c != 0);
                self.nz(v);
            }

            LSR_ACC => {
                let c = self.a & 1;
                self.a >>= 1;
                self.set_c(c != 0);
                self.nz(self.a);
            }
            LSR_ZP => {
                let a = self.zp();
                let mut v = self.read(a);
                let c = v & 1;
                v >>= 1;
                self.write(a, v);
                self.set_c(c != 0);
                self.nz(v);
            }
            LSR_ZPX => {
                let a = self.zpx();
                let mut v = self.read(a);
                let c = v & 1;
                v >>= 1;
                self.write(a, v);
                self.set_c(c != 0);
                self.nz(v);
            }
            LSR_ABS => {
                let a = self.abs();
                let mut v = self.read(a);
                let c = v & 1;
                v >>= 1;
                self.write(a, v);
                self.set_c(c != 0);
                self.nz(v);
            }
            LSR_ABX => {
                let a = self.absx();
                let mut v = self.read(a);
                let c = v & 1;
                v >>= 1;
                self.write(a, v);
                self.set_c(c != 0);
                self.nz(v);
            }

            ROL_ACC => {
                let old_c = self.p & FLAG_C != 0;
                let new_c = self.a >> 7 != 0;
                self.a = (self.a << 1) | (old_c as u8);
                self.set_c(new_c);
                self.nz(self.a);
            }
            ROL_ZP => {
                let a = self.zp();
                let mut v = self.read(a);
                let old_c = self.p & FLAG_C != 0;
                let new_c = v >> 7 != 0;
                v = (v << 1) | (old_c as u8);
                self.write(a, v);
                self.set_c(new_c);
                self.nz(v);
            }
            ROL_ZPX => {
                let a = self.zpx();
                let mut v = self.read(a);
                let old_c = self.p & FLAG_C != 0;
                let new_c = v >> 7 != 0;
                v = (v << 1) | (old_c as u8);
                self.write(a, v);
                self.set_c(new_c);
                self.nz(v);
            }
            ROL_ABS => {
                let a = self.abs();
                let mut v = self.read(a);
                let old_c = self.p & FLAG_C != 0;
                let new_c = v >> 7 != 0;
                v = (v << 1) | (old_c as u8);
                self.write(a, v);
                self.set_c(new_c);
                self.nz(v);
            }
            ROL_ABX => {
                let a = self.absx();
                let mut v = self.read(a);
                let old_c = self.p & FLAG_C != 0;
                let new_c = v >> 7 != 0;
                v = (v << 1) | (old_c as u8);
                self.write(a, v);
                self.set_c(new_c);
                self.nz(v);
            }

            ROR_ACC => {
                let old_c = self.p & FLAG_C != 0;
                let new_c = self.a & 1 != 0;
                self.a = (self.a >> 1) | ((old_c as u8) << 7);
                self.set_c(new_c);
                self.nz(self.a);
            }
            ROR_ZP => {
                let a = self.zp();
                let mut v = self.read(a);
                let old_c = self.p & FLAG_C != 0;
                let new_c = v & 1 != 0;
                v = (v >> 1) | ((old_c as u8) << 7);
                self.write(a, v);
                self.set_c(new_c);
                self.nz(v);
            }
            ROR_ZPX => {
                let a = self.zpx();
                let mut v = self.read(a);
                let old_c = self.p & FLAG_C != 0;
                let new_c = v & 1 != 0;
                v = (v >> 1) | ((old_c as u8) << 7);
                self.write(a, v);
                self.set_c(new_c);
                self.nz(v);
            }
            ROR_ABS => {
                let a = self.abs();
                let mut v = self.read(a);
                let old_c = self.p & FLAG_C != 0;
                let new_c = v & 1 != 0;
                v = (v >> 1) | ((old_c as u8) << 7);
                self.write(a, v);
                self.set_c(new_c);
                self.nz(v);
            }
            ROR_ABX => {
                let a = self.absx();
                let mut v = self.read(a);
                let old_c = self.p & FLAG_C != 0;
                let new_c = v & 1 != 0;
                v = (v >> 1) | ((old_c as u8) << 7);
                self.write(a, v);
                self.set_c(new_c);
                self.nz(v);
            }

            // ── Branches ──────────────────────────────────────────────────────
            BPL => {
                self.branch_with_page(self.p & FLAG_N == 0, &mut extra_cycles);
            }
            BMI => {
                self.branch_with_page(self.p & FLAG_N != 0, &mut extra_cycles);
            }
            BVC => {
                self.branch_with_page(self.p & FLAG_V == 0, &mut extra_cycles);
            }
            BVS => {
                self.branch_with_page(self.p & FLAG_V != 0, &mut extra_cycles);
            }
            BCC => {
                self.branch_with_page(self.p & FLAG_C == 0, &mut extra_cycles);
            }
            BCS => {
                self.branch_with_page(self.p & FLAG_C != 0, &mut extra_cycles);
            }
            BNE => {
                self.branch_with_page(self.p & FLAG_Z == 0, &mut extra_cycles);
            }
            BEQ => {
                self.branch_with_page(self.p & FLAG_Z != 0, &mut extra_cycles);
            }

            // ── Jumps / Subroutines / Returns ─────────────────────────────────
            JMP_ABS => {
                let a = self.abs();
                self.pc = a;
            }
            JMP_IND => {
                let ptr = self.abs();
                let lo = self.read(ptr);
                // 6502 page-wrap bug: high byte wraps within the same page
                let wrap = (ptr & 0xFF00) | (((ptr as u8).wrapping_add(1)) as u16);
                let hi = self.read(wrap);
                self.pc = lo as u16 | ((hi as u16) << 8);
            }
            JSR => {
                let addr = self.abs();
                trace!(
                    target: "emu6502::jsr",
                    "JSR ${:04X} → ${:04X}  [sp=${:02X}]",
                    self.pc - 3, addr, self.sp
                );
                let ret = self.pc - 1;
                self.push(((ret >> 8) & 0xFF) as u8);
                self.push((ret & 0xFF) as u8);
                self.pc = addr;
            }
            RTS => {
                let lo = self.pop() as u16;
                let hi = self.pop() as u16;
                self.pc = (lo | (hi << 8)).wrapping_add(1);
                trace!(
                    target: "emu6502::rts",
                    "RTS → ${:04X}  [sp=${:02X}]",
                    self.pc, self.sp
                );
            }
            RTI => {
                let p = self.pop();
                let lo = self.pop() as u16;
                let hi = self.pop() as u16;
                self.p = (p | FLAG_U) & !FLAG_B;
                self.pc = lo | (hi << 8);
            }

            // ── Stack & Status ────────────────────────────────────────────────
            PHA => {
                self.push(self.a);
            }
            PLA => {
                self.a = self.pop();
                self.nz(self.a);
            }
            PHP => {
                let s = self.p | FLAG_B | FLAG_U;
                self.push(s);
            }
            PLP => {
                self.p = self.pop();
                self.p |= FLAG_U;
                self.p &= !FLAG_B;
            }

            // ── Flag Control ──────────────────────────────────────────────────
            CLC => {
                self.p &= !FLAG_C;
            }
            SEC => {
                self.p |= FLAG_C;
            }
            CLI => {
                self.p &= !FLAG_I;
            }
            SEI => {
                self.p |= FLAG_I;
            }
            CLV => {
                self.p &= !FLAG_V;
            }
            CLD => {
                self.p &= !FLAG_D;
            }
            SED => {
                self.p |= FLAG_D;
            }

            // ── BRK / NOP ─────────────────────────────────────────────────────
            BRK => {
                trace!(
                    target: "emu6502::brk",
                    "BRK at PC=${:04X}, trap_brk={}",
                    instr_pc, self.trap_brk
                );
                if self.trap_brk {
                    self.halted = true;
                    trace!(target: "emu6502::halt", "CPU HALTED at ${:04X}", instr_pc);
                } else {
                    let pc = self.pc;
                    self.push(((pc >> 8) & 0xFF) as u8);
                    self.push((pc & 0xFF) as u8);
                    let s = self.p | FLAG_B | FLAG_U;
                    self.push(s);
                    self.p |= FLAG_I;
                    self.pc = self.read_word(vec_irq());
                }
            }
            NOP => {}

            _ => {
                panic!("unimplemented opcode {:02X} @ ${:04X}", op, instr_pc);
            }
        }
        self.cycles += Self::base_cycles(op) as u64 + extra_cycles as u64;
    }

    // ── Addressing modes & helpers ────────────────────────────────────────────
    fn fetch8(&mut self) -> u8 {
        let b = self.mem[self.pc as usize];
        self.pc = self.pc.wrapping_add(1);
        b
    }
    fn push(&mut self, v: u8) {
        let addr = 0x0100 | self.sp as u16;
        self.mem[addr as usize] = v;
        self.sp = self.sp.wrapping_sub(1);
    }
    fn pop(&mut self) -> u8 {
        self.sp = self.sp.wrapping_add(1);
        let addr = 0x0100 | self.sp as u16;
        self.mem[addr as usize]
    }
    fn read_word(&self, addr: u16) -> u16 {
        let lo = self.read(addr) as u16;
        let hi = self.read(addr.wrapping_add(1)) as u16;
        lo | (hi << 8)
    }
    fn imm(&mut self) -> u8 {
        self.fetch8()
    }
    fn zp(&mut self) -> u16 {
        self.fetch8() as u16
    }
    fn zpx(&mut self) -> u16 {
        self.fetch8().wrapping_add(self.x) as u16
    }
    fn zpy(&mut self) -> u16 {
        self.fetch8().wrapping_add(self.y) as u16
    }
    fn abs(&mut self) -> u16 {
        let lo = self.fetch8() as u16;
        let hi = self.fetch8() as u16;
        lo | (hi << 8)
    }
    fn absx(&mut self) -> u16 {
        self.abs().wrapping_add(self.x as u16)
    }
    fn absy(&mut self) -> u16 {
        self.abs().wrapping_add(self.y as u16)
    }
    fn absx_pc(&mut self) -> (u16, bool) {
        let base = self.abs();
        let addr = base.wrapping_add(self.x as u16);
        (addr, (base & 0xFF00) != (addr & 0xFF00))
    }
    fn absy_pc(&mut self) -> (u16, bool) {
        let base = self.abs();
        let addr = base.wrapping_add(self.y as u16);
        (addr, (base & 0xFF00) != (addr & 0xFF00))
    }
    fn indx(&mut self) -> u16 {
        let zp = self.fetch8().wrapping_add(self.x);
        let lo = self.read(zp as u16) as u16;
        let hi = self.read(zp.wrapping_add(1) as u16) as u16;
        lo | (hi << 8)
    }
    fn indy(&mut self) -> u16 {
        let zp = self.fetch8();
        let lo = self.read(zp as u16) as u16;
        let hi = self.read((zp.wrapping_add(1)) as u16) as u16;
        (lo | (hi << 8)).wrapping_add(self.y as u16)
    }
    fn indy_pc(&mut self) -> (u16, bool) {
        let zp = self.fetch8();
        let lo = self.read(zp as u16) as u16;
        let hi = self.read((zp.wrapping_add(1)) as u16) as u16;
        let base = lo | (hi << 8);
        let addr = base.wrapping_add(self.y as u16);
        (addr, (base & 0xFF00) != (addr & 0xFF00))
    }
    fn branch(&mut self, cond: bool) -> (bool, bool) {
        let off = self.fetch8();
        if cond {
            let old_pc = self.pc;
            let off = (off as i8) as i16;
            self.pc = self.pc.wrapping_add(off as u16);
            let crossed = (old_pc & 0xFF00) != (self.pc & 0xFF00);
            return (true, crossed);
        }
        (false, false)
    }
    fn branch_with_page(&mut self, cond: bool, extra: &mut u8) -> bool {
        let (taken, cross) = self.branch(cond);
        if taken {
            *extra += 1;
            if cross {
                *extra += 1;
            }
        }
        taken
    }

    // ── Cycle counts ─────────────────────────────────────────────────────────
    fn base_cycles(op: u8) -> u8 {
        use opcodes::*;
        match op {
            // 2-cycle: immediate reads, implied/accumulator, branches (base)
            LDA_IMM | LDX_IMM | LDY_IMM
            | ADC_IMM | SBC_IMM | SBC_IMM_ALT
            | AND_IMM | ORA_IMM | EOR_IMM
            | CMP_IMM | CPX_IMM | CPY_IMM
            | ASL_ACC | LSR_ACC | ROL_ACC | ROR_ACC
            | TAX | TXA | TAY | TYA | TSX | TXS
            | INX | INY | DEX | DEY
            | CLC | SEC | CLI | SEI | CLV | CLD | SED
            | NOP | BRK
            | BPL | BMI | BVC | BVS | BCC | BCS | BNE | BEQ => 2,

            // 3-cycle: zero-page reads/writes, JMP abs
            LDA_ZP | LDX_ZP | LDY_ZP
            | STA_ZP | STX_ZP | STY_ZP
            | ADC_ZP | SBC_ZP | AND_ZP | ORA_ZP | EOR_ZP
            | CMP_ZP | CPX_ZP | CPY_ZP | BIT_ZP
            | PHA | PLA | PHP | PLP
            | JMP_ABS => 3,

            // 4-cycle: absolute reads, zero-page indexed reads
            LDA_ABS | LDX_ABS | LDY_ABS
            | ADC_ABS | SBC_ABS | AND_ABS | ORA_ABS | EOR_ABS
            | CMP_ABS | CPX_ABS | CPY_ABS | BIT_ABS
            | LDA_ZPX | LDX_ZPY | LDY_ZPX
            | STA_ZPX | STX_ZPY | STY_ZPX
            | ADC_ZPX | SBC_ZPX | AND_ZPX | ORA_ZPX | EOR_ZPX | CMP_ZPX
            | LDA_ABX | LDA_ABY | LDX_ABY | LDY_ABX  // +1 on page cross counted separately
            | ADC_ABX | ADC_ABY | SBC_ABX | SBC_ABY
            | AND_ABX | AND_ABY | ORA_ABX | ORA_ABY | EOR_ABX | EOR_ABY
            | CMP_ABX | CMP_ABY
            | STA_ABS | STX_ABS | STY_ABS => 4,

            // 5-cycle: zero-page RMW, JMP indirect, (zp),Y reads
            INC_ZP | DEC_ZP | ASL_ZP | LSR_ZP | ROL_ZP | ROR_ZP
            | INC_ZPX | DEC_ZPX | ASL_ZPX | LSR_ZPX | ROL_ZPX | ROR_ZPX
            | LDA_INY | ADC_INY | SBC_INY | AND_INY | ORA_INY | EOR_INY | CMP_INY  // +1 page cross
            | JMP_IND => 5,

            // 6-cycle: absolute RMW, absolute stores indexed, (zp,X) reads, JSR, PLA, PLP
            INC_ABS | DEC_ABS | ASL_ABS | LSR_ABS | ROL_ABS | ROR_ABS
            | STA_ABX | STA_ABY
            | LDA_INX | ADC_INX | SBC_INX | AND_INX | ORA_INX | EOR_INX | CMP_INX
            | STA_INX | STA_INY
            | JSR | RTS | RTI => 6,

            // 7-cycle: absolute indexed RMW
            INC_ABX | DEC_ABX | ASL_ABX | LSR_ABX | ROL_ABX | ROR_ABX => 7,

            _ => 2,
        }
    }

    fn read(&self, addr: u16) -> u8 {
        for (a, cb) in &self.read_hooks {
            if *a == addr {
                return (cb)(self, addr);
            }
        }
        self.mem[addr as usize]
    }
    fn write(&mut self, addr: u16, v: u8) {
        // Resolve hook index first to avoid aliasing mutable borrow
        let hit = self.write_hooks.iter().position(|(a, _)| *a == addr);
        if let Some(i) = hit {
            let ptr: *const (u16, Box<dyn Fn(&mut Cpu, u16, u8)>) = &self.write_hooks[i];
            unsafe {
                ((*ptr).1)(self, addr, v);
            }
            return;
        }
        self.mem[addr as usize] = v;
    }
    fn interrupt(&mut self, vector: u16) {
        let pc = self.pc;
        self.push(((pc >> 8) & 0xFF) as u8);
        self.push((pc & 0xFF) as u8);
        let s = (self.p & !FLAG_B) | FLAG_U;
        self.push(s);
        self.p |= FLAG_I;
        self.pc = self.read_word(vector);
        if self.pc == 0 {
            self.halted = true; // safety: if vector empty and trap_brk
        }
        self.cycles += 7;
    }

    // ── Flag helpers & arithmetic ──────────────────────────────────────────────
    fn set_c(&mut self, c: bool) {
        if c {
            self.p |= FLAG_C;
        } else {
            self.p &= !FLAG_C;
        }
    }
    fn set_v(&mut self, v: bool) {
        if v {
            self.p |= FLAG_V;
        } else {
            self.p &= !FLAG_V;
        }
    }
    fn nz(&mut self, v: u8) {
        if v == 0 {
            self.p |= FLAG_Z;
        } else {
            self.p &= !FLAG_Z;
        }
        if v & 0x80 != 0 {
            self.p |= FLAG_N;
        } else {
            self.p &= !FLAG_N;
        }
    }
    fn adc(&mut self, v: u8) {
        let carry = (self.p & FLAG_C) != 0;
        let a = self.a;
        let (sum1, c1) = a.overflowing_add(v);
        let (sum2, c2) = sum1.overflowing_add(carry as u8);
        let overflow = ((a ^ sum2) & (v ^ sum2) & 0x80) != 0;
        let mut result = sum2;
        let mut c_out = c1 || c2;
        if self.p & FLAG_D != 0 {
            let mut lo = (a & 0x0F) + (v & 0x0F) + (carry as u8);
            let mut hi = (a >> 4) + (v >> 4);
            if lo > 9 {
                lo = lo.wrapping_add(6);
                hi = hi.wrapping_add(1);
            }
            if hi > 9 {
                hi = hi.wrapping_add(6);
                c_out = true;
            }
            result = (lo & 0x0F) | ((hi & 0x0F) << 4);
        }
        self.a = result;
        self.set_c(c_out);
        self.set_v(overflow);
        self.nz(self.a);
    }
    fn sbc(&mut self, v: u8) {
        let carry = (self.p & FLAG_C) != 0;
        let a = self.a;
        let vb = !v;
        let (sum1, c1) = a.overflowing_add(vb);
        let (sum2, c2) = sum1.overflowing_add(carry as u8);
        let overflow = ((a ^ sum2) & (!v ^ sum2) & 0x80) != 0;
        let mut result = sum2;
        let mut c_out = c1 || c2;
        if self.p & FLAG_D != 0 {
            let lo_a = (a & 0x0F) as i16;
            let hi_a = (a >> 4) as i16;
            let lo_v = (v & 0x0F) as i16;
            let hi_v = (v >> 4) as i16;
            let borrow_in = (!carry) as i16;
            let mut lo = lo_a - lo_v - borrow_in;
            let mut borrow = 0;
            if lo < 0 {
                lo -= 6;
                lo += 16;
                borrow = 1;
            }
            let mut hi = hi_a - hi_v - borrow;
            if hi < 0 {
                hi -= 6;
                hi += 16;
                c_out = false;
            } else {
                c_out = true;
            }
            result = ((hi as u8) << 4) | ((lo as u8) & 0x0F);
        }
        self.a = result;
        self.set_c(c_out);
        self.set_v(overflow);
        self.nz(self.a);
    }
    fn cmp(&mut self, reg: u8, v: u8) {
        let r = reg.wrapping_sub(v);
        self.set_c(reg >= v);
        self.nz(r);
    }
    fn bit(&mut self, v: u8) {
        let r = self.a & v;
        if r == 0 {
            self.p |= FLAG_Z;
        } else {
            self.p &= !FLAG_Z;
        }
        if v & 0x40 != 0 {
            self.p |= FLAG_V;
        } else {
            self.p &= !FLAG_V;
        }
        if v & 0x80 != 0 {
            self.p |= FLAG_N;
        } else {
            self.p &= !FLAG_N;
        }
    }
}

// ── Interrupt vector addresses ────────────────────────────────────────────────
fn vec_nmi() -> u16 {
    0xFFFA
}
fn vec_irq() -> u16 {
    0xFFFE
}

impl Cpu {
    pub fn request_irq(&mut self) {
        self.irq_pending = true;
    }
    pub fn request_nmi(&mut self) {
        self.nmi_pending = true;
    }
    pub fn disable_brk_trap(&mut self) {
        self.trap_brk = false;
    }
}

// ── Unit tests ────────────────────────────────────────────────────────────────
#[cfg(test)]
mod tests {
    use super::*;

    fn base_cpu() -> Cpu {
        let mut c = Cpu::new();
        c.mem[0xFFFC] = 0x00;
        c.mem[0xFFFD] = 0x08;
        c
    }

    #[test]
    fn load_store() {
        let mut c = base_cpu();
        // LDA #$10 ; STA $02 ; LDA $02 ; BRK
        c.load(0x0800, &[LDA_IMM, 0x10, STA_ZP, 0x02, LDA_ZP, 0x02, BRK]);
        c.reset();
        while !c.halted {
            c.step();
        }
        assert_eq!(c.a, 0x10);
        assert_eq!(c.mem[0x0002], 0x10);
    }

    #[test]
    fn adc_sbc() {
        let mut c = base_cpu();
        // LDA #5 ; ADC #3 -> 8 ; SEC ; SBC #1 -> 7 ; BRK
        c.load(
            0x0800,
            &[LDA_IMM, 0x05, ADC_IMM, 0x03, SEC, SBC_IMM, 0x01, BRK],
        );
        c.reset();
        while !c.halted {
            c.step();
        }
        assert_eq!(c.a, 0x07);
    }

    #[test]
    fn branches() {
        let mut c = base_cpu();
        // LDA #1 ; BNE +1 (skip BRK) ; BRK ; LDA #5 ; BRK
        c.load(0x0800, &[LDA_IMM, 0x01, BNE, 0x01, BRK, LDA_IMM, 0x05, BRK]);
        c.reset();
        while !c.halted {
            c.step();
        }
        assert_eq!(c.a, 0x05);
    }

    #[test]
    fn compare() {
        let mut c = base_cpu();
        // LDA #5 ; CMP #3 ; BRK
        c.load(0x0800, &[LDA_IMM, 0x05, CMP_IMM, 0x03, BRK]);
        c.reset();
        while !c.halted {
            c.step();
        }
        assert!(c.p & FLAG_C != 0);
    }

    #[test]
    fn decimal_adc() {
        let mut c = base_cpu();
        c.disable_brk_trap();
        c.mem[0xFFFE] = 0x00;
        c.mem[0xFFFF] = 0x10;
        // SED ; LDA #$15 ; ADC #$27 -> $42 BCD ; BRK
        c.load(0x0800, &[SED, LDA_IMM, 0x15, ADC_IMM, 0x27, BRK]);
        c.reset();
        while !c.halted {
            c.step();
            if c.cycles > 50 {
                break;
            }
        }
        assert_eq!(c.a, 0x42);
    }

    #[test]
    fn irq_basic() {
        let mut c = base_cpu();
        // IRQ vector -> handler: LDA #$33 ; BRK (trap halt)
        c.mem[0xFFFE] = 0x00;
        c.mem[0xFFFF] = 0x09;
        c.load(0x0900, &[LDA_IMM, 0x33, BRK]);
        // CLI + NOPs
        c.load(0x0800, &[CLI, NOP, NOP]);
        c.reset();
        c.step(); // CLI clears I
        c.request_irq();
        for _ in 0..30 {
            if c.halted {
                break;
            }
            c.step();
        }
        assert!(c.halted, "IRQ handler did not BRK");
        assert_eq!(c.a, 0x33);
    }

    #[test]
    fn write_hook() {
        use std::cell::RefCell;
        use std::rc::Rc;
        let mut c = base_cpu();
        let out = Rc::new(RefCell::new(Vec::<u8>::new()));
        let out2 = out.clone();
        c.hook_write(0xF001, move |_cpu, _addr, v| {
            out2.borrow_mut().push(v);
        });
        // LDA #'A' ; STA $F001 ; BRK
        c.load(0x0800, &[LDA_IMM, b'A', STA_ABS, 0x01, 0xF0, BRK]);
        c.reset();
        c.run_until_brk(50).unwrap();
        assert_eq!(out.borrow().as_slice(), b"A");
    }

    #[test]
    fn page_cross_indexed_read_cycles() {
        let mut c = base_cpu();
        // LDX #1 ; LDA $10FF,X (crosses into $1100) ; BRK
        c.load(0x0800, &[LDX_IMM, 0x01, LDA_ABX, 0xFF, 0x10, BRK]);
        c.mem[0x1100] = 0x55;
        c.reset();
        while !c.halted {
            c.step();
        }
        // LDX #=2, LDA abs,X base 4 +1 page cross, BRK=2 => total 9
        assert_eq!(c.cycles, 9);
        assert_eq!(c.a, 0x55);
    }

    #[test]
    fn branch_page_cross_cycle() {
        let mut c = base_cpu();
        // Code at $08FE: LDA #1 ; BNE +2 (crosses to $0903) ; BRK ; NOP ; BRK
        c.load(0x08FE, &[LDA_IMM, 0x01, BNE, 0x02, BRK, NOP, BRK]);
        c.mem[0xFFFC] = 0xFE;
        c.mem[0xFFFD] = 0x08;
        c.reset();
        while !c.halted {
            c.step();
            if c.cycles > 30 {
                break;
            }
        }
        assert_eq!(c.cycles, 7);
    }
}
