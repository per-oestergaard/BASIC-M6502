use crate::Cpu;

/// Simple harness to run small 6502 BASIC-related snippets with a captured character output.
/// It hooks writes to a configured I/O address (default 0xF001) and appends bytes to an output buffer.
use std::rc::Rc; use std::cell::RefCell; use std::collections::VecDeque;
pub struct BasicHarness {
    pub cpu: Cpu,
    out_addr: u16,
    output: Rc<RefCell<Vec<u8>>>,
    input: Rc<RefCell<VecDeque<u8>>>,
}

impl BasicHarness {
    pub fn new() -> Self { Self::with_addr(0x0002) }
    pub fn with_addr(out_addr: u16) -> Self {
        let mut cpu = Cpu::new();
        cpu.mem[0xFFFC] = 0x00; // reset vector lo
        cpu.mem[0xFFFD] = 0x08; // reset vector hi -> $0800
    let mut harness = Self { cpu, out_addr, output: Rc::new(RefCell::new(Vec::new())), input: Rc::new(RefCell::new(VecDeque::new())) };
        harness.install_hook();
        harness
    }
    fn install_hook(&mut self){
    let addr = self.out_addr; let sink = self.output.clone();
    self.cpu.hook_write(addr, move |_c,_a,v| { sink.borrow_mut().push(v); });
    }
    pub fn load_and_run(&mut self, addr: u16, bytes: &[u8], cycle_budget: u64) -> Result<(), &'static str> {
        self.cpu.load(addr, bytes); self.cpu.reset(); self.cpu.run_until_brk(cycle_budget)
    }
    pub fn output_bytes(&self) -> Vec<u8> { self.output.borrow().clone() }
    pub fn output_string(&self) -> String { String::from_utf8_lossy(&self.output.borrow()).into_owned() }
    pub fn queue_input_str(&mut self, s:&str){ for b in s.bytes(){ self.input.borrow_mut().push_back(b); } }
    pub fn install_input_hook(&mut self, addr:u16){ let q=self.input.clone(); self.cpu.hook_read(addr, move |_c,_a|{ if let Some(ch)=q.borrow_mut().pop_front(){ ch } else { 0 } }); }
    pub fn load_binary_and_run(&mut self, path:&str, load_addr:u16, cycle_budget:u64)->std::io::Result<Result<(), &'static str>>{ use std::fs; let data=fs::read(path)?; self.cpu.load(load_addr,&data); self.cpu.reset(); Ok(self.cpu.run_until_brk(cycle_budget)) }
    pub fn run_until_output_suffix(&mut self, suffix:&str, cycle_budget:u64)->Result<(), &'static str>{
        let target = suffix.as_bytes();
        let start = self.cpu.cycles;
        while (self.cpu.cycles-start) < cycle_budget {
            if self.cpu.halted { return Ok(()); }
            self.cpu.step();
            let buf = self.output.borrow();
            if buf.len() >= target.len() && &buf[buf.len()-target.len()..]==target { return Ok(()); }
        }
        Err("timeout waiting for output suffix")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn hello_output(){
        let mut h = BasicHarness::new();
        // Program at $0800: output "HELLO" by successive loads and stores, then BRK.
        let program = [ // STA $0002 sequences (8D 02 00)
            0xA9, b'H', 0x8D, 0x02, 0x00,
            0xA9, b'E', 0x8D, 0x02, 0x00,
            0xA9, b'L', 0x8D, 0x02, 0x00,
            0xA9, b'L', 0x8D, 0x02, 0x00,
            0xA9, b'O', 0x8D, 0x02, 0x00,
            0x00
        ];
        h.load_and_run(0x0800, &program, 10_000).unwrap();
    assert_eq!(h.output_string(), "HELLO");
    }
    #[test]
    fn suffix_wait(){
        let mut h = BasicHarness::new();
        let program = [
            0xA9, b'O', 0x8D, 0x02, 0x00,
            0xA9, b'K', 0x8D, 0x02, 0x00,
            0x00
        ];
        h.load_and_run(0x0800,&program, 100).unwrap();
        assert_eq!(h.output_string(), "OK");
    }
    #[test]
    fn maybe_basic_ready(){
        let path = "disasm/orig/basic.bin";
        if std::path::Path::new(path).exists(){
            let mut h=BasicHarness::new();
            if let Ok(res)=h.load_binary_and_run(path,0x0800, 5_000_000){ let _=res; let out=h.output_string(); assert!(out.contains("READY")||out.contains("Ok"), "No READY in output ({} bytes)", out.len()); }
        }
    }
}
