use crate::Cpu;

use std::cell::RefCell;
use std::collections::VecDeque;
/// Simple harness to run small 6502 BASIC-related snippets with a captured character output.
/// It hooks writes to a configured I/O address (default 0xF001) and appends bytes to an output buffer.
use std::rc::Rc;
use tracing::trace;

fn format_output_char(byte: u8) -> char {
    let ch = byte & 0x7F;
    if (0x20..0x7F).contains(&ch) {
        ch as char
    } else {
        '.'
    }
}
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
        // MEMSIZ ($0076-$0077): top of usable RAM — set to $3000 (12 KB)
        cpu.mem[0x0076] = 0x00;
        cpu.mem[0x0077] = 0x30;

        // TXTTAB ($006A-$006B): prime so INIT's `INC TXTTAB` wraps lo $FF→$00 and
        // the BNE is not taken, causing INIT to also do `INC TXTTAB+1` ($07→$08).
        // Result: TXTTAB = $0800 (start of program text, just after the binary).
        cpu.mem[0x006A] = 0xFF;
        cpu.mem[0x006B] = 0x07;

        // ── Shared state: output buffer & input line queue ───────────────────
        let output: Rc<RefCell<Vec<u8>>> = Rc::new(RefCell::new(Vec::new()));
        // Each entry is one complete input line as GETLN would return it:
        // high-bit-set characters in BUF, with X set to the character count.
        // INLIN appends the trailing NUL itself for REALIO=4.
        let lines: Rc<RefCell<VecDeque<Vec<u8>>>> = Rc::new(RefCell::new(VecDeque::new()));
        let trace_steps: Rc<RefCell<usize>> = Rc::new(RefCell::new(0));
        let after_run: Rc<RefCell<bool>> = Rc::new(RefCell::new(false));
        let run_output_start: Rc<RefCell<Option<usize>>> = Rc::new(RefCell::new(None));

        {
            let mut q = lines.borrow_mut();
            // Apple BASIC cold-start prompts for memory size and terminal width
            // before it reaches the main READY loop. Supply an explicit memory
            // size so the interpreter does not perform its destructive RAM probe
            // against the flat emulator memory image, then leave terminal width
            // blank to accept the default.
            q.push_back(b"12288".iter().map(|b| b | 0x80).collect());
            q.push_back(b"72".iter().map(|b| b | 0x80).collect());
            for line in program.lines() {
                let bytes: Vec<u8> = line
                    .to_ascii_uppercase()
                    .bytes()
                    .map(|b| b | 0x80)
                    .collect();
                q.push_back(bytes);
            }
            // Feed "RUN" after the program
            let run_bytes: Vec<u8> = b"RUN".iter().map(|b| b | 0x80).collect();
            q.push_back(run_bytes);
        }

        // ── I/O hooks via exec traps ─────────────────────────────────────────
        // $FD67 = Apple II GETLN (CQINLN): line-at-a-time input.
        // INLIN for REALIO=4 calls JSR $FD67, not the character-by-character $FD0C.
        // Convention: write the line (with high bits set) into BUF ($0200),
        // set X = character count, set carry, then RTS.
        let lq = lines.clone();
        let trace_after_input = trace_steps.clone();
        let run_state = after_run.clone();
        let run_output_mark = run_output_start.clone();
        let output_for_run_mark = output.clone();
        cpu.hook_exec(0xFD67, move |cpu| {
            if let Some(line) = lq.borrow_mut().pop_front() {
                let ascii: Vec<u8> = line.iter().map(|&b| b & 0x7F).collect();
                let text = String::from_utf8_lossy(&ascii);
                trace!(target: "emu6502::harness", bytes = line.len(), input = %text, "GETLN hook");
                if text == "RUN" {
                    *run_state.borrow_mut() = true;
                    *run_output_mark.borrow_mut() = Some(output_for_run_mark.borrow().len());
                    let mut ptr = (cpu.mem[0x006A] as u16) | ((cpu.mem[0x006B] as u16) << 8);
                    let vartab = (cpu.mem[0x006C] as u16) | ((cpu.mem[0x006D] as u16) << 8);
                    trace!(target: "emu6502::harness", txttab = ptr, vartab, "RUN starting");
                    let mut guard = 0usize;
                    while ptr != 0 && ptr < vartab && guard < 8 {
                        let next = (cpu.mem[ptr as usize] as u16)
                            | ((cpu.mem[ptr as usize + 1] as u16) << 8);
                        let line_no = (cpu.mem[ptr as usize + 2] as u16)
                            | ((cpu.mem[ptr as usize + 3] as u16) << 8);
                        let mut bytes = Vec::new();
                        let mut cur = ptr as usize + 4;
                        while cur < cpu.mem.len() && cpu.mem[cur] != 0 {
                            bytes.push(cpu.mem[cur]);
                            cur += 1;
                        }
                        trace!(target: "emu6502::harness", line_no, next, bytes = ?bytes, "queued BASIC line");
                        if next == 0 || next <= ptr {
                            break;
                        }
                        ptr = next;
                        guard += 1;
                    }
                }
                let len = line.len().min(255);
                cpu.mem[0x0200..0x02FF].fill(0);
                for (i, &b) in line[..len].iter().enumerate() {
                    cpu.mem[0x0200 + i] = b;
                }
                cpu.x = len as u8;
                cpu.p |= 0x01; // set carry (success)
                *trace_after_input.borrow_mut() = if text == "RUN" { 20000 } else { 1500 };
            } else {
                // No more input - this should cause the test to complete or timeout
                trace!(target: "emu6502::harness", "GETLN exhausted input; halting CPU");
                cpu.halted = true;
            }
            cpu.trap_rts();
        });

        // Temporary diagnostic hook: stop on the first BASIC error and print X.
        cpu.hook_exec(0x09CD, move |cpu| {
            let error_name = match cpu.x {
                0 => "NF",
                2 => "SN",
                4 => "RG",
                6 => "OD",
                8 => "FC",
                10 => "OV",
                12 => "OM",
                14 => "US",
                16 => "BS",
                18 => "DD",
                20 => "/0",
                22 => "ID",
                24 => "TM",
                26 => "LS",
                30 => "ST",
                32 => "CN",
                34 => "UF",
                _ => "?",
            };
            trace!(
                target: "emu6502::harness",
                error_code = cpu.x,
                error_name,
                a = cpu.a,
                txtptr = (cpu.mem[0x0001] as u16) | ((cpu.mem[0x0002] as u16) << 8),
                chrget = ?[
                    cpu.mem[0x00C3],
                    cpu.mem[0x00C4],
                    cpu.mem[0x00C5],
                    cpu.mem[0x00C6],
                    cpu.mem[0x00C7],
                    cpu.mem[0x00C8],
                ],
                "BASIC error"
            );
            cpu.halted = true;
        });

        // $FECD = Apple II COUT: primary character output path used by OUTDO.
        let out1 = output.clone();
        let out1_count = Rc::new(RefCell::new(0usize));
        let oc1 = out1_count.clone();
        let after_run_out1 = after_run.clone();
        cpu.hook_exec(0xFECD, move |cpu| {
            *oc1.borrow_mut() += 1;
            if *oc1.borrow() <= 5 {
                trace!(target: "emu6502::harness", call = *oc1.borrow(), a = cpu.a, ch = %format_output_char(cpu.a), "COUT");
            }
            if *after_run_out1.borrow() {
                trace!(target: "emu6502::harness", a = cpu.a, ch = %format_output_char(cpu.a), "POST-RUN COUT");
            }
            out1.borrow_mut().push(cpu.a & 0x7F);
            cpu.trap_rts();
        });

        // $FDED = Apple II COUT1: secondary output path (used by some print routines).
        let out2 = output.clone();
        let out2_count = Rc::new(RefCell::new(0usize));
        let oc2 = out2_count.clone();
        let after_run_out2 = after_run.clone();
        cpu.hook_exec(0xFDED, move |cpu| {
            *oc2.borrow_mut() += 1;
            if *oc2.borrow() <= 10 {
                trace!(target: "emu6502::harness", call = *oc2.borrow(), a = cpu.a, ch = %format_output_char(cpu.a), "COUT1");
            }
            if *after_run_out2.borrow() {
                trace!(target: "emu6502::harness", a = cpu.a, ch = %format_output_char(cpu.a), "POST-RUN COUT1");
            }
            out2.borrow_mut().push(cpu.a & 0x7F);
            cpu.trap_rts();
        });

        // ── Set up entry registers ────────────────────────────────────────────
        // Start at $0000 which contains JMP INIT
        cpu.a = 0;
        cpu.x = 0;
        cpu.y = 0;
        cpu.sp = 0xFF;
        cpu.pc = 0x0000; // START: JMP INIT

        // ── Run until "]" (BASIC's post-RUN prompt) appears in output ─────────
        // Apple Basic prints "]\n" after each command including after "RUN" finishes.
        // We stop when we see that, or on timeout.
        let start_cycles = cpu.cycles;
        let mut last_output_len = 0usize;
        let exit_reason = loop {
            if cpu.halted {
                break "halted";
            }
            if (cpu.cycles - start_cycles) >= max_cycles {
                break "timeout";
            }

            if *trace_steps.borrow() > 0 {
                let remaining = *trace_steps.borrow();
                let pc = cpu.pc;
                let op = cpu.mem[pc as usize];
                let in_scrub_loop = (0x0AEE..=0x0AF7).contains(&pc);
                if !in_scrub_loop || remaining % 40 == 0 {
                    trace!(
                        target: "emu6502::harness",
                        pc,
                        op,
                        a = cpu.a,
                        x = cpu.x,
                        y = cpu.y,
                        sp = cpu.sp,
                        p = cpu.p,
                        txtptr = (cpu.mem[0x0001] as u16) | ((cpu.mem[0x0002] as u16) << 8),
                        "step"
                    );
                }
                *trace_steps.borrow_mut() = remaining - 1;
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
                    break "prompt";
                }
            }
        };

        trace!(target: "emu6502::harness", exit_reason, pc = cpu.pc, cycles = cpu.cycles - start_cycles, "run loop exited");

        // Convert output to string: Apple II CR ($0D) → newline, strip non-printables
        let raw = output.borrow().clone();
        let s = normalize_basic_output(
            &raw.iter()
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
                .collect::<String>(),
        );

        if let Some(start) = *run_output_start.borrow() {
            let post_run = normalize_basic_output(
                &raw[start..]
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
                    .collect::<String>(),
            );
            let trimmed = post_run.trim();
            if let Some(without_ok) = trimmed.strip_suffix("\n\nOK") {
                return Ok(without_ok.trim().to_string());
            }
            if let Some(without_ok) = trimmed.strip_suffix("\nOK") {
                return Ok(without_ok.trim().to_string());
            }
            if !trimmed.is_empty() {
                return Ok(trimmed.to_string());
            }
        }

        // Strip the cold-start transcript and return only program output.
        // Apple BASIC prints a stable startup preamble ending with a standalone
        // "OK" before user program execution begins.
        if let Some(ok_pos) = s.rfind("\nOK\n") {
            let after_ok = s[ok_pos + 4..].trim();
            if !after_ok.is_empty() {
                return Ok(after_ok.to_string());
            }
        }

        if let Some(ok_pos) = s.rfind("OK\n") {
            let after_ok = s[ok_pos + 3..].trim();
            if !after_ok.is_empty() {
                return Ok(after_ok.to_string());
            }
        }

        Ok(s.trim().to_string())
    }
}

fn normalize_basic_output(text: &str) -> String {
    text.split('\n')
        .map(|line| line.trim_end_matches(' '))
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Once;
    use tracing::info;

    static INIT_TRACING: Once = Once::new();

    fn init_tracing() {
        INIT_TRACING.call_once(|| {
            tracing_subscriber::fmt()
                .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
                .with_test_writer()
                .init();
        });
    }

    #[test]
    fn hello_output() {
        init_tracing();
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
        init_tracing();
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
        init_tracing();
        let bin_path = "../build/original/basic.bin";
        if !std::path::Path::new(bin_path).exists() {
            info!(
                path = bin_path,
                "skipping trace_init because binary was not found"
            );
            return;
        }
        use std::cell::RefCell;
        use std::rc::Rc;

        let data = std::fs::read(bin_path).unwrap();
        let mut cpu = crate::Cpu::new();
        cpu.mem[..data.len().min(65536)].copy_from_slice(&data[..data.len().min(65536)]);
        // MEMSIZ = $3000
        cpu.mem[0x0076] = 0x00;
        cpu.mem[0x0077] = 0x30;
        // TXTTAB preset to $0800 so INIT's INC makes it $0801 (hi byte $08, non-zero)
        cpu.mem[0x006A] = 0xFF; // lo byte wraps after INC: $FF -> $00
        cpu.mem[0x006B] = 0x07; // hi byte = $07; after lo wraps BASIC does INC hi -> $08
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
                info!(
                    pc = pc_before,
                    opcode = data.get(pc_before as usize).copied().unwrap_or(0),
                    "CPU halted"
                );
                break;
            }
            if *cqinln_hit.borrow() {
                info!(step = instr_count, pc = cpu.pc, "CQINLN hit");
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
        info!(
            instructions = instr_count,
            ?halted_at,
            pc = cpu.pc,
            a = cpu.a,
            x = cpu.x,
            y = cpu.y,
            sp = cpu.sp,
            "trace_init summary"
        );
        info!(txttab_lo = cpu.mem[0x6A], txttab_hi = cpu.mem[0x6B], chrget_lo = cpu.mem[0xBA], chrget_hi = cpu.mem[0xBB], samples = ?sample_pcs, output_len = raw.len(), output = %&s[..s.len().min(200)], "trace_init state");
    }

    #[test]
    fn maybe_basic_ready() {
        init_tracing();
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
