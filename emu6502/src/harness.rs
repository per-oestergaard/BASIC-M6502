use crate::Cpu;

use std::cell::RefCell;
use std::collections::VecDeque;
/// Simple harness to run small 6502 BASIC-related snippets with a captured character output.
/// It hooks writes to a configured I/O address (default 0xF001) and appends bytes to an output buffer.
use std::rc::Rc;
pub struct BasicHarness {
    pub cpu: Cpu,
    out_addr: u16,
    output: Rc<RefCell<Vec<u8>>>,
    input: Rc<RefCell<VecDeque<u8>>>,
}

impl BasicHarness {
    pub fn new() -> Self {
        Self::with_addr(0x0002)
    }
    pub fn with_addr(out_addr: u16) -> Self {
        let mut cpu = Cpu::new();
        cpu.mem[0xFFFC] = 0x00; // reset vector lo
        cpu.mem[0xFFFD] = 0x08; // reset vector hi -> $0800
        let mut harness = Self {
            cpu,
            out_addr,
            output: Rc::new(RefCell::new(Vec::new())),
            input: Rc::new(RefCell::new(VecDeque::new())),
        };
        harness.install_hook();
        harness
    }
    fn install_hook(&mut self) {
        let addr = self.out_addr;
        let sink = self.output.clone();
        self.cpu.hook_write(addr, move |_c, _a, v| {
            sink.borrow_mut().push(v);
        });
    }
    pub fn load_and_run(
        &mut self,
        addr: u16,
        bytes: &[u8],
        cycle_budget: u64,
    ) -> Result<(), &'static str> {
        self.cpu.load(addr, bytes);
        self.cpu.reset();
        self.cpu.run_until_brk(cycle_budget)
    }
    pub fn output_bytes(&self) -> Vec<u8> {
        self.output.borrow().clone()
    }
    pub fn output_string(&self) -> String {
        String::from_utf8_lossy(&self.output.borrow()).into_owned()
    }
    pub fn queue_input_str(&mut self, s: &str) {
        for b in s.bytes() {
            self.input.borrow_mut().push_back(b);
        }
    }
    pub fn install_input_hook(&mut self, addr: u16) {
        let q = self.input.clone();
        self.cpu.hook_read(addr, move |_c, _a| {
            if let Some(ch) = q.borrow_mut().pop_front() {
                ch
            } else {
                0
            }
        });
    }
    pub fn load_binary_and_run(
        &mut self,
        path: &str,
        load_addr: u16,
        cycle_budget: u64,
    ) -> std::io::Result<Result<(), &'static str>> {
        use std::fs;
        let data = fs::read(path)?;
        self.cpu.load(load_addr, &data);
        self.cpu.reset();
        Ok(self.cpu.run_until_brk(cycle_budget))
    }
    pub fn run_until_output_suffix(
        &mut self,
        suffix: &str,
        cycle_budget: u64,
    ) -> Result<(), &'static str> {
        let target = suffix.as_bytes();
        let start = self.cpu.cycles;
        while (self.cpu.cycles - start) < cycle_budget {
            if self.cpu.halted {
                return Ok(());
            }
            self.cpu.step();
            let buf = self.output.borrow();
            if buf.len() >= target.len() && &buf[buf.len() - target.len()..] == target {
                return Ok(());
            }
        }
        Err("timeout waiting for output suffix")
    }

    /// Run Apple II Microsoft BASIC with the given BASIC source program.
    ///
    /// Loads `binary_path` as a flat $0000-based image, initialises the interpreter,
    /// feeds the program (line by line then "RUN") as keyboard input, and returns
    /// the captured ASCII output after "]\n" (BASIC's ">" prompt after RUN finishes)
    /// or after `max_cycles` without a result.
    ///
    /// Must be called on a fresh harness; the harness instance is consumed by setup.
    pub fn run_apple_ii_basic(
        binary_path: &str,
        program: &str,
        max_cycles: u64,
    ) -> Result<String, String> {
        use std::fs;

        // ── Load the flat binary image at $0000 ──────────────────────────────
        let data = fs::read(binary_path).map_err(|e| format!("Failed to read binary: {}", e))?;

        let mut cpu = Cpu::new();
        // Load binary starting at $0000 (it includes ZP reservations then code at $0800)
        let load_len = data.len().min(65536);
        cpu.mem[..load_len].copy_from_slice(&data[..load_len]);

        // ── Pre-initialise BASIC variables ──────────────────────────────────
        // MEMSIZ ($0074-$0075): top of usable RAM — set to $3000 (12 KB)
        cpu.mem[0x0074] = 0x00;
        cpu.mem[0x0075] = 0x30;

        // TXTTAB ($0068-$0069): prime so INIT's `INC TXTTAB` wraps lo $FF→$00 and
        // the BNE is not taken, causing INIT to also do `INC TXTTAB+1` ($07→$08).
        // Result: TXTTAB = $0800 (start of program text, just after the binary).
        cpu.mem[0x0068] = 0xFF;
        cpu.mem[0x0069] = 0x07;

        // ── Shared state: output buffer & input line queue ───────────────────
        let output: Rc<RefCell<Vec<u8>>> = Rc::new(RefCell::new(Vec::new()));
        // Each entry is one complete input line (bytes with bit7 set, ending with $8D CR).
        let lines: Rc<RefCell<VecDeque<Vec<u8>>>> = Rc::new(RefCell::new(VecDeque::new()));

        {
            let mut q = lines.borrow_mut();
            for line in program.lines() {
                let mut bytes: Vec<u8> = line
                    .to_ascii_uppercase()
                    .bytes()
                    .map(|b| b | 0x80)
                    .collect();
                bytes.push(0x8D); // Apple II CR (bit7 set)
                q.push_back(bytes);
            }
            // Feed "RUN" after the program
            let mut run_bytes: Vec<u8> = b"RUN".iter().map(|b| b | 0x80).collect();
            run_bytes.push(0x8D);
            q.push_back(run_bytes);
        }

        // ── I/O hooks via exec traps ─────────────────────────────────────────
        // $FD67 = Apple II GETLN (CQINLN): line-at-a-time input.
        // INLIN for REALIO=4 calls JSR $FD67, not the character-by-character $FD0C.
        // Convention: write the line (with high bits set) into BUF ($0200),
        // set X = character count, set carry, then RTS.
        let lq = lines.clone();
        cpu.hook_exec(0xFD67, move |cpu| {
            let line = lq
                .borrow_mut()
                .pop_front()
                .unwrap_or_else(|| vec![0x8D]); // empty CR if nothing queued
            let len = line.len().min(255);
            for (i, &b) in line[..len].iter().enumerate() {
                cpu.mem[0x0200 + i] = b;
            }
            cpu.x = len as u8;
            cpu.p |= 0x01; // set carry (success)
            cpu.trap_rts();
        });

        // $FECD = Apple II COUT: primary character output path used by OUTDO.
        let out1 = output.clone();
        cpu.hook_exec(0xFECD, move |cpu| {
            out1.borrow_mut().push(cpu.a & 0x7F);
            cpu.trap_rts();
        });

        // $FDED = Apple II COUT1: secondary output path (used by some print routines).
        let out2 = output.clone();
        cpu.hook_exec(0xFDED, move |cpu| {
            out2.borrow_mut().push(cpu.a & 0x7F);
            cpu.trap_rts();
        });

        // ── Set up entry registers as Apple II firmware would ────────────────
        // INIT ($1E89) expects: X = CHRGET template length (INIT−INITAT = $1E89−$1E6C = $1D),
        // Y = terminal width (40), A = 0.
        cpu.a = 0;
        cpu.x = 0x1D; // 29 bytes: copy INITAT template to ZP CHRGET at $00B1
        cpu.y = 40;
        cpu.sp = 0xFF;
        cpu.pc = 0x1E89; // INIT cold-start

        // ── Run until "]" (BASIC's post-RUN prompt) appears in output ─────────
        // Apple Basic prints "]\n" after each command including after "RUN" finishes.
        // We stop when we see that, or on timeout.
        let start_cycles = cpu.cycles;
        let mut last_output_len = 0usize;

        loop {
            if cpu.halted || (cpu.cycles - start_cycles) >= max_cycles {
                break;
            }
            cpu.step();

            // Check for "]" (prompt after RUN completes) in output
            let buf = output.borrow();
            let new_len = buf.len();
            if new_len > last_output_len {
                last_output_len = new_len;
                // "]" followed by CR is the BASIC prompt after execution
                let s: &[u8] = &buf;
                if s.windows(2).any(|w| w == b"]\r" || w == b"]\n") {
                    break;
                }
            }
        }

        // Convert output to string: Apple II CR ($0D) → newline, strip non-printables
        let raw = output.borrow().clone();
        let s: String = raw
            .iter()
            .map(|&b| {
                let c = b & 0x7F;
                if c == 0x0D {
                    '\n'
                } else if c >= 0x20 && c < 0x7F {
                    c as char
                } else {
                    '\0'
                }
            })
            .filter(|&c| c != '\0')
            .collect();

        // Extract just the portion after "RUN\r" echo — the actual program output.
        // Find the last "]" prompt to locate start of RUN output.
        if let Some(pos) = s.rfind(']') {
            // Everything between the line containing "RUN" and the final "]"
            let before_last_prompt = &s[..pos];
            // Find where "RUN" echo appears and take everything after it
            if let Some(run_pos) = before_last_prompt.rfind("RUN") {
                let after_run = before_last_prompt[run_pos + 3..].trim_start_matches('\n');
                return Ok(after_run.trim_end_matches('\n').to_string());
            }
            return Ok(before_last_prompt.trim().to_string());
        }

        Ok(s.trim().to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn hello_output() {
        let mut h = BasicHarness::new();
        // Program at $0800: output "HELLO" by successive loads and stores, then BRK.
        let program = [
            // STA $0002 sequences (8D 02 00)
            0xA9, b'H', 0x8D, 0x02, 0x00, 0xA9, b'E', 0x8D, 0x02, 0x00, 0xA9, b'L', 0x8D, 0x02,
            0x00, 0xA9, b'L', 0x8D, 0x02, 0x00, 0xA9, b'O', 0x8D, 0x02, 0x00, 0x00,
        ];
        h.load_and_run(0x0800, &program, 10_000).unwrap();
        assert_eq!(h.output_string(), "HELLO");
    }
    #[test]
    fn suffix_wait() {
        let mut h = BasicHarness::new();
        let program = [
            0xA9, b'O', 0x8D, 0x02, 0x00, 0xA9, b'K', 0x8D, 0x02, 0x00, 0x00,
        ];
        h.load_and_run(0x0800, &program, 100).unwrap();
        assert_eq!(h.output_string(), "OK");
    }
    /// Trace the first N instructions of BASIC cold start to understand startup flow.
    /// Run with: cargo test trace_init -- --nocapture
    #[test]
    fn trace_init() {
        let bin_path = "../build/original/basic.bin";
        if !std::path::Path::new(bin_path).exists() {
            println!("SKIP: binary not found");
            return;
        }
        use std::cell::RefCell;
        use std::rc::Rc;

        let data = std::fs::read(bin_path).unwrap();
        let mut cpu = crate::Cpu::new();
        cpu.mem[..data.len().min(65536)].copy_from_slice(&data[..data.len().min(65536)]);
        // MEMSIZ = $3000
        cpu.mem[0x0074] = 0x00;
        cpu.mem[0x0075] = 0x30;
        // TXTTAB preset to $0800 so INIT's INC makes it $0801 (hi byte $08, non-zero)
        cpu.mem[0x0068] = 0xFF; // hi byte wraps after INC: $00 + 1 = $01 → wait, INC lo
        cpu.mem[0x0069] = 0x07; // hi byte = $07; after INC lo ($FF→$00) BASIC does INC hi → $08
                                // Actually: INIT does INC TXTTAB (lo only), BNE QROOM (skip INC hi if no carry)
                                // To get non-zero hi: set TXTTAB lo=$FF so INC wraps to $00 (Z=1), BNE not taken,
                                // then INC TXTTAB+1: $07+1=$08. TXTTAB = $0800.
        let output: Rc<RefCell<Vec<u8>>> = Rc::new(RefCell::new(Vec::new()));
        let out_a = output.clone();
        let out_b = output.clone();
        cpu.hook_exec(0xFDED, move |cpu| {
            out_a.borrow_mut().push(cpu.a & 0x7F);
            cpu.trap_rts();
        });
        cpu.hook_exec(0xFECD, move |cpu| {
            out_b.borrow_mut().push(cpu.a & 0x7F);
            cpu.trap_rts();
        });
        cpu.hook_exec(0x0006, |cpu| {
            cpu.trap_rts();
        }); // RDYJSR no-op
        let cqinln_hit = Rc::new(RefCell::new(false));
        let cq = cqinln_hit.clone();
        cpu.hook_exec(0xFD67, move |cpu| {
            *cq.borrow_mut() = true;
            // Feed "10 PRINT \"HI\"\r"
            let s = b"10 PRINT \"HI\"";
            for (i, &b) in s.iter().enumerate() {
                cpu.mem[0x0200 + i] = b | 0x80;
            }
            cpu.mem[0x0200 + s.len()] = 0x8D;
            cpu.x = (s.len() + 1) as u8;
            cpu.p |= 0x01;
            cpu.trap_rts();
        });
        cpu.disable_brk_trap();
        cpu.a = 0x00;
        cpu.x = 0x1D;
        cpu.y = 40;
        cpu.sp = 0xFF;
        cpu.pc = 0x1E89; // INIT
        let max_instructions = 500_000u64;
        let mut instr_count = 0u64;
        let mut halted_at: Option<u16> = None;
        let _last_pcs: std::collections::VecDeque<u16> =
            std::collections::VecDeque::with_capacity(20);
        let mut sample_pcs: Vec<u16> = Vec::new();
        // Step through instructions (not cycles)
        while instr_count < max_instructions {
            if cpu.halted {
                halted_at = Some(cpu.pc);
                break;
            }
            let pc_before = cpu.pc;
            cpu.step();
            if cpu.halted && halted_at.is_none() {
                halted_at = Some(pc_before);
                println!(
                    "HALTED at PC=${:04X}, opcode=${:02X}",
                    pc_before,
                    data.get(pc_before as usize).copied().unwrap_or(0)
                );
                break;
            }
            if *cqinln_hit.borrow() {
                println!(
                    "CQINLN ($FD67) hit at step {}! PC after hook = ${:04X}",
                    instr_count, cpu.pc
                );
                *cqinln_hit.borrow_mut() = false;
            }
            if instr_count % 50_000 == 0 {
                sample_pcs.push(cpu.pc);
            }
            instr_count += 1;
        }
        let raw = output.borrow().clone();
        let s: String = raw
            .iter()
            .map(|&b| {
                let c = b & 0x7F;
                if c == 0x0D {
                    '\n'
                } else if c >= 0x20 && c < 0x7F {
                    c as char
                } else {
                    '.'
                }
            })
            .collect();
        println!("Instructions executed: {}", instr_count);
        println!("Halted: {:?}", halted_at);
        println!("PC at end: ${:04X}", cpu.pc);
        println!(
            "A=${:02X} X=${:02X} Y=${:02X} SP=${:02X}",
            cpu.a, cpu.x, cpu.y, cpu.sp
        );
        println!(
            "TXTTAB (ZP $68-$69): ${:02X} ${:02X}",
            cpu.mem[0x68], cpu.mem[0x69]
        );
        println!(
            "CHRGET parse ptr ($BA-$BB): ${:02X} ${:02X}",
            cpu.mem[0xBA], cpu.mem[0xBB]
        );
        println!(
            "PC samples every 50k: {:?}",
            sample_pcs
                .iter()
                .map(|p| format!("${:04X}", p))
                .collect::<Vec<_>>()
        );
        println!("Output ({} bytes): {:?}", raw.len(), &s[..s.len().min(200)]);
    }

    #[test]
    fn maybe_basic_ready() {
        let path = "disasm/orig/basic.bin";
        if std::path::Path::new(path).exists() {
            let mut h = BasicHarness::new();
            if let Ok(res) = h.load_binary_and_run(path, 0x0800, 5_000_000) {
                let _ = res;
                let out = h.output_string();
                assert!(
                    out.contains("READY") || out.contains("Ok"),
                    "No READY in output ({} bytes)",
                    out.len()
                );
            }
        }
    }
}
