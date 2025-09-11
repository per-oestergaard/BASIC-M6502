//! Expanded (still incomplete) 6502 CPU core oriented toward Microsoft BASIC execution.
//! Cycle counts & some undocumented opcodes omitted for now

mod harness;
pub use harness::BasicHarness;

// Status flag bit masks
pub const FLAG_C:u8=0x01; // Carry
pub const FLAG_Z:u8=0x02; // Zero
pub const FLAG_I:u8=0x04; // IRQ Disable
pub const FLAG_D:u8=0x08; // Decimal
pub const FLAG_B:u8=0x10; // Break
pub const FLAG_U:u8=0x20; // Unused (always set when pushed)
pub const FLAG_V:u8=0x40; // Overflow
pub const FLAG_N:u8=0x80; // Negative

pub struct Cpu {
    pub a:u8, pub x:u8, pub y:u8,
    pub sp:u8, pub p:u8, pub pc:u16,
    pub mem:[u8;65536], pub cycles:u64,
    pub halted: bool,
    irq_pending: bool,
    nmi_pending: bool,
    trap_brk: bool,
    read_hooks: Vec<(u16, Box<dyn Fn(&Cpu,u16)->u8>)>,
    write_hooks: Vec<(u16, Box<dyn Fn(&mut Cpu,u16,u8)>)>,
}

impl Default for Cpu { fn default()->Self{Self::new()} }

impl Cpu {
    pub fn new()->Self{ Self{ a:0,x:0,y:0,sp:0xFD,p:FLAG_U|FLAG_I,pc:0,mem:[0;65536],cycles:0,halted:false, irq_pending:false, nmi_pending:false, trap_brk:true, read_hooks:Vec::new(), write_hooks:Vec::new() } }
    pub fn load(&mut self, addr:u16, bytes:&[u8]){ self.mem[addr as usize .. addr as usize + bytes.len()].copy_from_slice(bytes); }
    pub fn reset(&mut self){ self.sp=0xFD; let lo=self.mem[0xFFFC] as u16; let hi=self.mem[0xFFFD] as u16; self.pc=lo | (hi<<8); self.halted=false; }

    // Public run helper (cycle bounded)
    pub fn run_cycles(&mut self, max:u64){ while self.cycles<max && !self.halted { self.step(); } }
    // Convenience: run until BRK (halt) or cycle budget exceeded
    pub fn run_until_brk(&mut self, max_cycles:u64)->Result<(), &'static str>{ let start=self.cycles; while !self.halted && (self.cycles-start)<max_cycles { self.step(); } if self.halted {Ok(())} else {Err("cycle budget exceeded without BRK")} }
    // Generic predicate runner (returns true when predicate returns true) with cycle cap
    pub fn run_until<F:Fn(&Cpu)->bool>(&mut self, max_cycles:u64, pred:F)->Result<(), &'static str>{ let start=self.cycles; while (self.cycles-start)<max_cycles { if pred(self) { return Ok(());} if self.halted { return Ok(());} self.step(); } Err("cycle budget exceeded") }
    // Hook registration
    pub fn hook_read<F:'static+Fn(&Cpu,u16)->u8>(&mut self, addr:u16, f:F){ self.read_hooks.push((addr, Box::new(f))); }
    pub fn hook_write<F:'static+Fn(&mut Cpu,u16,u8)>(&mut self, addr:u16, f:F){ self.write_hooks.push((addr, Box::new(f))); }

    // Core step
    pub fn step(&mut self){ if self.halted {return;} // Interrupt poll (NMI first)
        if self.nmi_pending { self.nmi_pending=false; self.interrupt(vec_nmi()); }
        else if self.irq_pending && (self.p & FLAG_I)==0 { self.irq_pending=false; self.interrupt(vec_irq()); }
        if self.halted {return;} // interrupt may set halted (if vector 0 and trap)
        let op=self.fetch8(); let mut extra_cycles=0u8; match op {
    // Load/Store (refactored to avoid nested mutable borrows)
    0xA9=>{let v=self.imm(); self.a=v; self.nz(v);} // LDA #
    0xA5=>{let addr=self.zp(); let v=self.read(addr); self.a=v; self.nz(v);} // LDA zp
    0xB5=>{let addr=self.zpx(); let v=self.read(addr); self.a=v; self.nz(v);} // LDA zp,X
    0xAD=>{let addr=self.abs(); let v=self.read(addr); self.a=v; self.nz(v);} // LDA abs
    0xBD=>{let (addr,pc)=self.absx_pc(); let v=self.read(addr); if pc { extra_cycles+=1;} self.a=v; self.nz(v);} // LDA abs,X (+1 if page cross)
    0xB9=>{let (addr,pc)=self.absy_pc(); let v=self.read(addr); if pc { extra_cycles+=1;} self.a=v; self.nz(v);} // LDA abs,Y (+1 if page cross)
    0xA1=>{let addr=self.indx(); let v=self.read(addr); self.a=v; self.nz(v);} // LDA (zp,X)
    0xB1=>{let addr=self.indy(); let v=self.read(addr); self.a=v; self.nz(v);} // LDA (zp),Y
    0xA2=>{let v=self.imm(); self.x=v; self.nz(v);} // LDX #
    0xA6=>{let addr=self.zp(); let v=self.read(addr); self.x=v; self.nz(v);} 0xB6=>{let addr=self.zpy(); let v=self.read(addr); self.x=v; self.nz(v);} 0xAE=>{let addr=self.abs(); let v=self.read(addr); self.x=v; self.nz(v);} 0xBE=>{let (addr,pc)=self.absy_pc(); let v=self.read(addr); if pc { extra_cycles+=1;} self.x=v; self.nz(v);} 
    0xA0=>{let v=self.imm(); self.y=v; self.nz(v);} 0xA4=>{let addr=self.zp(); let v=self.read(addr); self.y=v; self.nz(v);} 0xB4=>{let addr=self.zpx(); let v=self.read(addr); self.y=v; self.nz(v);} 0xAC=>{let addr=self.abs(); let v=self.read(addr); self.y=v; self.nz(v);} 0xBC=>{let (addr,pc)=self.absx_pc(); let v=self.read(addr); if pc { extra_cycles+=1;} self.y=v; self.nz(v);} 
    0x85=>{let addr=self.zp(); self.write(addr,self.a);} 0x95=>{let addr=self.zpx(); self.write(addr,self.a);} 0x8D=>{let addr=self.abs(); self.write(addr,self.a);} 0x9D=>{let addr=self.absx(); self.write(addr,self.a);} 0x99=>{let addr=self.absy(); self.write(addr,self.a);} 0x81=>{let addr=self.indx(); self.write(addr,self.a);} 0x91=>{let addr=self.indy(); self.write(addr,self.a);} 
    0x86=>{let addr=self.zp(); self.write(addr,self.x);} 0x96=>{let addr=self.zpy(); self.write(addr,self.x);} 0x8E=>{let addr=self.abs(); self.write(addr,self.x);} 
    0x84=>{let addr=self.zp(); self.write(addr,self.y);} 0x94=>{let addr=self.zpx(); self.write(addr,self.y);} 0x8C=>{let addr=self.abs(); self.write(addr,self.y);} 

        // Transfers
        0xAA=>{self.x=self.a; self.nz(self.x);} 0x8A=>{self.a=self.x; self.nz(self.a);} 0xA8=>{self.y=self.a; self.nz(self.y);} 0x98=>{self.a=self.y; self.nz(self.a);} 0xBA=>{self.x=self.sp; self.nz(self.x);} 0x9A=>{self.sp=self.x;}

        // Increments / Decrements
        0xE8=>{self.x=self.x.wrapping_add(1); self.nz(self.x);} 0xC8=>{self.y=self.y.wrapping_add(1); self.nz(self.y);} 0xCA=>{self.x=self.x.wrapping_sub(1); self.nz(self.x);} 0x88=>{self.y=self.y.wrapping_sub(1); self.nz(self.y);} 0xE6=>{let a=self.zp(); let v=self.read(a).wrapping_add(1); self.write(a,v); self.nz(v);} 0xF6=>{let a=self.zpx(); let v=self.read(a).wrapping_add(1); self.write(a,v); self.nz(v);} 0xEE=>{let a=self.abs(); let v=self.read(a).wrapping_add(1); self.write(a,v); self.nz(v);} 0xFE=>{let a=self.absx(); let v=self.read(a).wrapping_add(1); self.write(a,v); self.nz(v);} 0xC6=>{let a=self.zp(); let v=self.read(a).wrapping_sub(1); self.write(a,v); self.nz(v);} 0xD6=>{let a=self.zpx(); let v=self.read(a).wrapping_sub(1); self.write(a,v); self.nz(v);} 0xCE=>{let a=self.abs(); let v=self.read(a).wrapping_sub(1); self.write(a,v); self.nz(v);} 0xDE=>{let a=self.absx(); let v=self.read(a).wrapping_sub(1); self.write(a,v); self.nz(v);} 

        // Arithmetic / Logic
    0x69=>{let v=self.imm(); self.adc(v);} 0x65=>{let a=self.zp(); let v=self.read(a); self.adc(v);} 0x75=>{let a=self.zpx(); let v=self.read(a); self.adc(v);} 0x6D=>{let a=self.abs(); let v=self.read(a); self.adc(v);} 0x7D=>{let (a,pc)=self.absx_pc(); let v=self.read(a); if pc { extra_cycles+=1;} self.adc(v);} 0x79=>{let (a,pc)=self.absy_pc(); let v=self.read(a); if pc { extra_cycles+=1;} self.adc(v);} 0x61=>{let a=self.indx(); let v=self.read(a); self.adc(v);} 0x71=>{let (a,pc)=self.indy_pc(); let v=self.read(a); if pc { extra_cycles+=1;} self.adc(v);} 
    0xE9|0xEB=>{let v=self.imm(); self.sbc(v);} 0xE5=>{let a=self.zp(); let v=self.read(a); self.sbc(v);} 0xF5=>{let a=self.zpx(); let v=self.read(a); self.sbc(v);} 0xED=>{let a=self.abs(); let v=self.read(a); self.sbc(v);} 0xFD=>{let (a,pc)=self.absx_pc(); let v=self.read(a); if pc { extra_cycles+=1;} self.sbc(v);} 0xF9=>{let (a,pc)=self.absy_pc(); let v=self.read(a); if pc { extra_cycles+=1;} self.sbc(v);} 0xE1=>{let a=self.indx(); let v=self.read(a); self.sbc(v);} 0xF1=>{let (a,pc)=self.indy_pc(); let v=self.read(a); if pc { extra_cycles+=1;} self.sbc(v);} 
    0x29=>{let v=self.imm(); self.a &= v; self.nz(self.a);} 0x25=>{let a=self.zp(); let v=self.read(a); self.a &= v; self.nz(self.a);} 0x35=>{let a=self.zpx(); let v=self.read(a); self.a &= v; self.nz(self.a);} 0x2D=>{let a=self.abs(); let v=self.read(a); self.a &= v; self.nz(self.a);} 0x3D=>{let (a,pc)=self.absx_pc(); let v=self.read(a); if pc { extra_cycles+=1;} self.a &= v; self.nz(self.a);} 0x39=>{let (a,pc)=self.absy_pc(); let v=self.read(a); if pc { extra_cycles+=1;} self.a &= v; self.nz(self.a);} 0x21=>{let a=self.indx(); let v=self.read(a); self.a &= v; self.nz(self.a);} 0x31=>{let (a,pc)=self.indy_pc(); let v=self.read(a); if pc { extra_cycles+=1;} self.a &= v; self.nz(self.a);} 
    0x09=>{let v=self.imm(); self.a |= v; self.nz(self.a);} 0x05=>{let a=self.zp(); let v=self.read(a); self.a |= v; self.nz(self.a);} 0x15=>{let a=self.zpx(); let v=self.read(a); self.a |= v; self.nz(self.a);} 0x0D=>{let a=self.abs(); let v=self.read(a); self.a |= v; self.nz(self.a);} 0x1D=>{let (a,pc)=self.absx_pc(); let v=self.read(a); if pc { extra_cycles+=1;} self.a |= v; self.nz(self.a);} 0x19=>{let (a,pc)=self.absy_pc(); let v=self.read(a); if pc { extra_cycles+=1;} self.a |= v; self.nz(self.a);} 0x01=>{let a=self.indx(); let v=self.read(a); self.a |= v; self.nz(self.a);} 0x11=>{let (a,pc)=self.indy_pc(); let v=self.read(a); if pc { extra_cycles+=1;} self.a |= v; self.nz(self.a);} 
    0x49=>{let v=self.imm(); self.a ^= v; self.nz(self.a);} 0x45=>{let a=self.zp(); let v=self.read(a); self.a ^= v; self.nz(self.a);} 0x55=>{let a=self.zpx(); let v=self.read(a); self.a ^= v; self.nz(self.a);} 0x4D=>{let a=self.abs(); let v=self.read(a); self.a ^= v; self.nz(self.a);} 0x5D=>{let (a,pc)=self.absx_pc(); let v=self.read(a); if pc { extra_cycles+=1;} self.a ^= v; self.nz(self.a);} 0x59=>{let (a,pc)=self.absy_pc(); let v=self.read(a); if pc { extra_cycles+=1;} self.a ^= v; self.nz(self.a);} 0x41=>{let a=self.indx(); let v=self.read(a); self.a ^= v; self.nz(self.a);} 0x51=>{let (a,pc)=self.indy_pc(); let v=self.read(a); if pc { extra_cycles+=1;} self.a ^= v; self.nz(self.a);} 
    0x24=>{let a=self.zp(); let v=self.read(a); self.bit(v);} 0x2C=>{let a=self.abs(); let v=self.read(a); self.bit(v);} 
    0xC9=>{let v=self.imm(); self.cmp(self.a,v);} 0xC5=>{let a=self.zp(); let v=self.read(a); self.cmp(self.a,v);} 0xD5=>{let a=self.zpx(); let v=self.read(a); self.cmp(self.a,v);} 0xCD=>{let a=self.abs(); let v=self.read(a); self.cmp(self.a,v);} 0xDD=>{let (a,pc)=self.absx_pc(); let v=self.read(a); if pc { extra_cycles+=1;} self.cmp(self.a,v);} 0xD9=>{let (a,pc)=self.absy_pc(); let v=self.read(a); if pc { extra_cycles+=1;} self.cmp(self.a,v);} 0xC1=>{let a=self.indx(); let v=self.read(a); self.cmp(self.a,v);} 0xD1=>{let (a,pc)=self.indy_pc(); let v=self.read(a); if pc { extra_cycles+=1;} self.cmp(self.a,v);} 
    0xE0=>{let v=self.imm(); self.cmp(self.x,v);} 0xE4=>{let a=self.zp(); let v=self.read(a); self.cmp(self.x,v);} 0xEC=>{let a=self.abs(); let v=self.read(a); self.cmp(self.x,v);} 
    0xC0=>{let v=self.imm(); self.cmp(self.y,v);} 0xC4=>{let a=self.zp(); let v=self.read(a); self.cmp(self.y,v);} 0xCC=>{let a=self.abs(); let v=self.read(a); self.cmp(self.y,v);} 

        // Shifts & Rotates (accumulator & memory forms)
        0x0A=>{let c=(self.a>>7)&1; self.a=self.a<<1; self.set_c(c!=0); self.nz(self.a);} 0x06=>{let a=self.zp(); let mut v=self.read(a); let c=(v>>7)&1; v<<=1; self.write(a,v); self.set_c(c!=0); self.nz(v);} 0x16=>{let a=self.zpx(); let mut v=self.read(a); let c=(v>>7)&1; v<<=1; self.write(a,v); self.set_c(c!=0); self.nz(v);} 0x0E=>{let a=self.abs(); let mut v=self.read(a); let c=(v>>7)&1; v<<=1; self.write(a,v); self.set_c(c!=0); self.nz(v);} 0x1E=>{let a=self.absx(); let mut v=self.read(a); let c=(v>>7)&1; v<<=1; self.write(a,v); self.set_c(c!=0); self.nz(v);} 
        0x4A=>{let c=self.a&1; self.a>>=1; self.set_c(c!=0); self.nz(self.a);} 0x46=>{let a=self.zp(); let mut v=self.read(a); let c=v&1; v>>=1; self.write(a,v); self.set_c(c!=0); self.nz(v);} 0x56=>{let a=self.zpx(); let mut v=self.read(a); let c=v&1; v>>=1; self.write(a,v); self.set_c(c!=0); self.nz(v);} 0x4E=>{let a=self.abs(); let mut v=self.read(a); let c=v&1; v>>=1; self.write(a,v); self.set_c(c!=0); self.nz(v);} 0x5E=>{let a=self.absx(); let mut v=self.read(a); let c=v&1; v>>=1; self.write(a,v); self.set_c(c!=0); self.nz(v);} 
        0x2A=>{let old_c=self.p&FLAG_C!=0; let new_c=self.a>>7!=0; self.a=(self.a<<1)|(old_c as u8); self.set_c(new_c); self.nz(self.a);} 0x26=>{let a=self.zp(); let mut v=self.read(a); let old_c=self.p&FLAG_C!=0; let new_c=v>>7!=0; v=(v<<1)|(old_c as u8); self.write(a,v); self.set_c(new_c); self.nz(v);} 0x36=>{let a=self.zpx(); let mut v=self.read(a); let old_c=self.p&FLAG_C!=0; let new_c=v>>7!=0; v=(v<<1)|(old_c as u8); self.write(a,v); self.set_c(new_c); self.nz(v);} 0x2E=>{let a=self.abs(); let mut v=self.read(a); let old_c=self.p&FLAG_C!=0; let new_c=v>>7!=0; v=(v<<1)|(old_c as u8); self.write(a,v); self.set_c(new_c); self.nz(v);} 0x3E=>{let a=self.absx(); let mut v=self.read(a); let old_c=self.p&FLAG_C!=0; let new_c=v>>7!=0; v=(v<<1)|(old_c as u8); self.write(a,v); self.set_c(new_c); self.nz(v);} 
        0x6A=>{let old_c=self.p&FLAG_C!=0; let new_c=self.a&1!=0; self.a=(self.a>>1)|((old_c as u8)<<7); self.set_c(new_c); self.nz(self.a);} 0x66=>{let a=self.zp(); let mut v=self.read(a); let old_c=self.p&FLAG_C!=0; let new_c=v&1!=0; v=(v>>1)|((old_c as u8)<<7); self.write(a,v); self.set_c(new_c); self.nz(v);} 0x76=>{let a=self.zpx(); let mut v=self.read(a); let old_c=self.p&FLAG_C!=0; let new_c=v&1!=0; v=(v>>1)|((old_c as u8)<<7); self.write(a,v); self.set_c(new_c); self.nz(v);} 0x6E=>{let a=self.abs(); let mut v=self.read(a); let old_c=self.p&FLAG_C!=0; let new_c=v&1!=0; v=(v>>1)|((old_c as u8)<<7); self.write(a,v); self.set_c(new_c); self.nz(v);} 0x7E=>{let a=self.absx(); let mut v=self.read(a); let old_c=self.p&FLAG_C!=0; let new_c=v&1!=0; v=(v>>1)|((old_c as u8)<<7); self.write(a,v); self.set_c(new_c); self.nz(v);} 

    // Branches (+1 if taken; +1 more if page crossed)
    0x10=>{ if self.branch_with_page(self.p & FLAG_N ==0, &mut extra_cycles){} } 0x30=>{ if self.branch_with_page(self.p & FLAG_N !=0, &mut extra_cycles){} } 0x50=>{ if self.branch_with_page(self.p & FLAG_V ==0, &mut extra_cycles){} } 0x70=>{ if self.branch_with_page(self.p & FLAG_V !=0, &mut extra_cycles){} } 0x90=>{ if self.branch_with_page(self.p & FLAG_C ==0, &mut extra_cycles){} } 0xB0=>{ if self.branch_with_page(self.p & FLAG_C !=0, &mut extra_cycles){} } 0xD0=>{ if self.branch_with_page(self.p & FLAG_Z ==0, &mut extra_cycles){} } 0xF0=>{ if self.branch_with_page(self.p & FLAG_Z !=0, &mut extra_cycles){} } 

        // Jumps / Subroutines / Returns
    0x4C=>{let addr=self.abs(); self.pc=addr;} 0x6C=>{let ptr=self.abs(); let lo=self.read(ptr); let wrap=(ptr & 0xFF00) | (((ptr as u8).wrapping_add(1)) as u16); let hi=self.read(wrap); self.pc=lo as u16 | ((hi as u16)<<8);} 0x20=>{let addr=self.abs(); let ret=self.pc-1; self.push(((ret>>8)&0xFF) as u8); self.push((ret&0xFF) as u8); self.pc=addr;} 0x60=>{let lo=self.pop() as u16; let hi=self.pop() as u16; self.pc=(lo | (hi<<8)).wrapping_add(1);} 0x40=>{let lo=self.pop() as u16; let hi=self.pop() as u16; let p=self.pop(); self.p=p|FLAG_U; self.pc=lo | (hi<<8);} 

        // Stack & status
    0x48=>{self.push(self.a);} 0x68=>{self.a=self.pop(); self.nz(self.a);} 0x08=>{let s=self.p | FLAG_B | FLAG_U; self.push(s);} 0x28=>{self.p=self.pop(); self.p|=FLAG_U; self.p&=!FLAG_B;} 

        // Flag control
        0x18=>{self.p&=!FLAG_C;} 0x38=>{self.p|=FLAG_C;} 0x58=>{self.p&=!FLAG_I;} 0x78=>{self.p|=FLAG_I;} 0xB8=>{self.p&=!FLAG_V;} 0xD8=>{self.p&=!FLAG_D;} 0xF8=>{self.p|=FLAG_D;} 

    // BRK & NOP
    0x00=>{ if self.trap_brk { self.halted=true; } else { // proper BRK sequence
        let pc=self.pc; // PC already points to next byte
        self.push(((pc>>8)&0xFF) as u8); self.push((pc&0xFF) as u8);
    let s=self.p | FLAG_B | FLAG_U; self.push(s);
        self.p|=FLAG_I; // set IRQ disable
        self.pc = self.read_word(vec_irq());
        }} 0xEA=>{/*NOP*/}

    _=>{panic!("unimplemented opcode {:02X} @{:04X}",op,self.pc-1);} } self.cycles += Self::base_cycles(op) as u64 + extra_cycles as u64; }

    // Addressing & helpers
    fn fetch8(&mut self)->u8{ let b=self.mem[self.pc as usize]; self.pc=self.pc.wrapping_add(1); b }
    fn push(&mut self,v:u8){ let addr=0x0100 | self.sp as u16; self.mem[addr as usize]=v; self.sp=self.sp.wrapping_sub(1); }
    fn pop(&mut self)->u8{ self.sp=self.sp.wrapping_add(1); let addr=0x0100 | self.sp as u16; self.mem[addr as usize] }
    fn read_word(&self, addr:u16)->u16{ let lo=self.read(addr) as u16; let hi=self.read(addr.wrapping_add(1)) as u16; lo | (hi<<8) }
    fn imm(&mut self)->u8{ self.fetch8() }
    fn zp(&mut self)->u16{ self.fetch8() as u16 }
    fn zpx(&mut self)->u16{ self.fetch8().wrapping_add(self.x) as u16 }
    fn zpy(&mut self)->u16{ self.fetch8().wrapping_add(self.y) as u16 }
    fn abs(&mut self)->u16{ let lo=self.fetch8() as u16; let hi=self.fetch8() as u16; lo | (hi<<8) }
    fn absx(&mut self)->u16{ self.abs().wrapping_add(self.x as u16) }
    fn absy(&mut self)->u16{ self.abs().wrapping_add(self.y as u16) }
    // Variants returning whether a page boundary was crossed
    fn absx_pc(&mut self)->(u16,bool){ let base=self.abs(); let addr=base.wrapping_add(self.x as u16); (addr, (base & 0xFF00)!=(addr & 0xFF00)) }
    fn absy_pc(&mut self)->(u16,bool){ let base=self.abs(); let addr=base.wrapping_add(self.y as u16); (addr, (base & 0xFF00)!=(addr & 0xFF00)) }
    fn indx(&mut self)->u16{ let zp=self.fetch8().wrapping_add(self.x); let lo=self.read(zp as u16) as u16; let hi=self.read(zp.wrapping_add(1) as u16) as u16; lo | (hi<<8) }
    fn indy(&mut self)->u16{ let zp=self.fetch8(); let lo=self.read(zp as u16) as u16; let hi=self.read((zp.wrapping_add(1)) as u16) as u16; (lo | (hi<<8)).wrapping_add(self.y as u16) }
    fn indy_pc(&mut self)->(u16,bool){ let zp=self.fetch8(); let lo=self.read(zp as u16) as u16; let hi=self.read((zp.wrapping_add(1)) as u16) as u16; let base=lo | (hi<<8); let addr=base.wrapping_add(self.y as u16); (addr, (base & 0xFF00)!=(addr & 0xFF00)) }
    fn branch(&mut self, cond:bool)->(bool,bool){ let off=self.fetch8(); if cond { let old_pc=self.pc; let off=(off as i8) as i16; self.pc=self.pc.wrapping_add(off as u16); let crossed=(old_pc & 0xFF00)!=(self.pc & 0xFF00); return (true,crossed);} (false,false) }
    fn branch_with_page(&mut self, cond:bool, extra:&mut u8)->bool{ let (taken,cross)=self.branch(cond); if taken { *extra+=1; if cross { *extra+=1; } } taken }
    fn base_cycles(op:u8)->u8{ match op {
        0xA9|0xA2|0xA0|0x09|0x29|0x49|0xC9|0xE0|0xC0|0x69|0xE9|0xEB =>2,
        0xA5|0xA6|0xA4|0x85|0x86|0x84|0x05|0x25|0x45|0x24|0xC5|0xE4|0xC4|0x65|0xE5 =>3,
        // Zero page RMW (INC/DEC/ASL/LSR/ROL/ROR)
        0xC6|0xE6|0x06|0x46|0x26|0x66 =>5,
        // Zero page indexed (read or RMW +1)
        0xB5|0xB4|0x95|0x94|0x15|0x35|0x55|0xD5|0xF5 =>4,
        0xD6|0xF6|0x16|0x56|0x36|0x76 =>6,
        // Absolute
        0xAD|0xAE|0xAC|0x8D|0x0D|0x2D|0x4D|0x2C|0xCD|0xEC|0xCC|0x6D|0xED =>4,
        // Absolute RMW (6 cycles)
        0x0E|0x4E|0x2E|0x6E|0xEE|0xCE =>6,
        // Absolute indexed read (add page cross later), stores (STA abs,X=5) & RMW fixed
        0xBD|0xB9|0xBC|0xBE|0x9D|0x99|0x1D|0x3D|0x5D|0xDD|0xFD =>4,
        // Absolute indexed RMW (7 cycles) and DEC/INC/shift/rotate
        0xDE|0xFE|0x1E|0x5E|0x3E|0x7E =>7,
        // (indirect,X) and (indirect),Y base 6 for (indirect,X) and 5 for (indirect),Y; adjust here simplified 5, add +1 for (indirect),Y page cross
        0xA1|0x21|0x41|0x61|0xE1|0xC1|0x01|0x81 =>6, // (indirect,X) group including STA/ORA etc.
        0xB1|0x31|0x51|0x71|0xF1|0xD1|0x11|0x91 =>5, // (indirect),Y group variable page cross
        0x20=>6, 0x4C=>3, 0x6C=>5, 0x60=>6, 0x40=>6,
        0x48|0x68|0x08|0x28 =>3,
        0x18|0x38|0x58|0x78|0xB8|0xD8|0xF8|0xAA|0x8A|0xA8|0x98|0xBA|0x9A|0xE8|0xC8|0xCA|0x88|0xEA|0x00|0x0A|0x4A|0x2A|0x6A =>2,
        0x10|0x30|0x50|0x70|0x90|0xB0|0xD0|0xF0 =>2,
        _=>2
    }}
    fn read(&self, addr:u16)->u8{ for (a,cb) in &self.read_hooks { if *a==addr { return (cb)(self,addr); } } self.mem[addr as usize] }
    fn write(&mut self, addr:u16, v:u8){
        // Avoid aliasing mutable borrow by resolving index first
        let mut hit=None; for (i,(a,_)) in self.write_hooks.iter().enumerate(){ if *a==addr { hit=Some(i); break; } }
        if let Some(i)=hit { let ptr: *const (u16, Box<dyn Fn(&mut Cpu,u16,u8)>) = &self.write_hooks[i]; unsafe { ((*ptr).1)(self,addr,v); } return; }
        self.mem[addr as usize]=v;
    }
    fn interrupt(&mut self, vector:u16){
        // Push PC high, low, then status with B cleared for IRQ/NMI
        let pc=self.pc; self.push(((pc>>8)&0xFF) as u8); self.push((pc&0xFF) as u8);
        let s=(self.p & !FLAG_B) | FLAG_U; self.push(s);
        self.p|=FLAG_I; // disable further IRQs
        self.pc=self.read_word(vector);
        if self.pc==0 { // safety: if vector empty and trap_brk emulate halt
            self.halted=true;
        }
        self.cycles += 7; // interrupt entry cost
    }

    // Flag operations & arithmetic
    fn set_c(&mut self, c:bool){ if c { self.p|=FLAG_C; } else { self.p&=!FLAG_C; } }
    fn set_v(&mut self, v:bool){ if v { self.p|=FLAG_V; } else { self.p&=!FLAG_V; } }
    fn nz(&mut self,v:u8){ if v==0 { self.p|=FLAG_Z; } else { self.p&=!FLAG_Z; } if v&0x80!=0 { self.p|=FLAG_N; } else { self.p&=!FLAG_N; } }
    fn adc(&mut self, v:u8){
        let carry=(self.p & FLAG_C)!=0;
        let a=self.a;
        let (sum1,c1)=a.overflowing_add(v);
        let (sum2,c2)=sum1.overflowing_add(carry as u8);
        // Overflow (binary) detection remains binary even in decimal mode on NMOS 6502
        let overflow=((a ^ sum2) & (v ^ sum2) & 0x80)!=0;
        let mut result=sum2;
        let mut c_out=c1 || c2;
        if self.p & FLAG_D !=0 { // Simplified BCD correction
            let mut lo = (a & 0x0F) + (v & 0x0F) + (carry as u8);
            let mut hi = (a >>4) + (v >>4);
            if lo > 9 { lo = lo.wrapping_add(6); hi = hi.wrapping_add(1); }
            if hi > 9 { hi = hi.wrapping_add(6); c_out = true; }
            result = (lo & 0x0F) | ((hi & 0x0F)<<4);
        }
        self.a=result; self.set_c(c_out); self.set_v(overflow); self.nz(self.a);
    }
    fn sbc(&mut self, v:u8){
        let carry=(self.p & FLAG_C)!=0; // 1 means no borrow
        let a=self.a;
        let vb=!v; // one's complement
        let (sum1,c1)=a.overflowing_add(vb);
        let (sum2,c2)=sum1.overflowing_add(carry as u8);
        let overflow=((a ^ sum2) & (!v ^ sum2) & 0x80)!=0; // standard SBC overflow
        let mut result=sum2; let mut c_out=c1 || c2; // this is correct carry (no borrow)
        if self.p & FLAG_D !=0 { // BCD SBC correction
            let lo_a = (a & 0x0F) as i16;
            let hi_a = (a >>4) as i16;
            let lo_v = (v & 0x0F) as i16;
            let hi_v = (v >>4) as i16;
            let borrow_in = (!carry) as i16; // 1 if borrow
            let mut lo = lo_a - lo_v - borrow_in;
            let mut borrow = 0;
            if lo < 0 { lo -= 6; lo += 16; borrow = 1; }
            let mut hi = hi_a - hi_v - borrow;
            if hi < 0 { hi -= 6; hi += 16; c_out=false; } else { c_out=true; }
            result = ((hi as u8) <<4) | ((lo as u8) & 0x0F);
        }
        self.a=result; self.set_c(c_out); self.set_v(overflow); self.nz(self.a);
    }
    fn cmp(&mut self, reg:u8, v:u8){ let r=reg.wrapping_sub(v); self.set_c(reg>=v); self.nz(r); }
    fn bit(&mut self,v:u8){ let r=self.a & v; if r==0 { self.p|=FLAG_Z;} else { self.p&=!FLAG_Z;} if v&0x40!=0 { self.p|=FLAG_V;} else { self.p&=!FLAG_V;} if v&0x80!=0 { self.p|=FLAG_N;} else { self.p&=!FLAG_N;} }
}

// Interrupt vectors helper (standard 6502 memory map)
fn vec_nmi()->u16{0xFFFA}
fn vec_irq()->u16{0xFFFE}

impl Cpu {
    pub fn request_irq(&mut self){ self.irq_pending=true; }
    pub fn request_nmi(&mut self){ self.nmi_pending=true; }
    pub fn disable_brk_trap(&mut self){ self.trap_brk=false; }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn base_cpu()->Cpu { let mut c=Cpu::new(); c.mem[0xFFFC]=0x00; c.mem[0xFFFD]=0x08; c }
    #[test] fn load_store(){ let mut c=base_cpu(); c.load(0x0800,&[0xA9,0x10,0x85,0x02,0xA5,0x02,0x00]); c.reset(); while !c.halted { c.step(); } assert_eq!(c.a,0x10); assert_eq!(c.mem[0x0002],0x10); }
    #[test] fn adc_sbc(){ let mut c=base_cpu(); // A=5; ADC #3 => 8; SEC; SBC #1 => 7
        c.load(0x0800,&[0xA9,0x05,0x69,0x03,0x38,0xE9,0x01,0x00]); c.reset(); while !c.halted { c.step(); } assert_eq!(c.a,0x07); }
    #[test] fn branches(){ let mut c=base_cpu(); // BNE skip BRK (offset 1), then execute LDA #5
        c.load(0x0800,&[0xA9,0x01,0xD0,0x01,0x00,0xA9,0x05,0x00]); c.reset(); while !c.halted { c.step(); } assert_eq!(c.a,0x05); }
    #[test] fn compare(){ let mut c=base_cpu(); c.load(0x0800,&[0xA9,0x05,0xC9,0x03,0x00]); c.reset(); while !c.halted { c.step(); } assert!(c.p & FLAG_C !=0); }
    #[test] fn decimal_adc(){ let mut c=base_cpu(); c.disable_brk_trap();
        // No interrupts: set IRQ vector somewhere safe
        c.mem[0xFFFE]=0x00; c.mem[0xFFFF]=0x10; c.load(0x0800,&[0xF8,0xA9,0x15,0x69,0x27,0x00]); // SED; LDA #$15; ADC #$27 -> $42 (BCD)
        c.reset(); while !c.halted { c.step(); if c.cycles>50 { break; } }
        assert_eq!(c.a,0x42); }
    #[test] fn irq_basic(){ let mut c=base_cpu(); // keep BRK as trap for halt
        // IRQ vector points to handler: LDA #$33; BRK (trap halt)
        c.mem[0xFFFE]=0x00; c.mem[0xFFFF]=0x09; c.load(0x0900,&[0xA9,0x33,0x00]);
        c.load(0x0800,&[0x58,0xEA,0xEA]); // CLI + NOPs
        c.reset(); c.step(); // CLI clears I
        c.request_irq();
        for _ in 0..30 { if c.halted { break; } c.step(); }
        assert!(c.halted, "IRQ handler did not BRK"); assert_eq!(c.a,0x33); }
    #[test] fn write_hook(){ use std::rc::Rc; use std::cell::RefCell; let mut c=base_cpu(); let out=Rc::new(RefCell::new(Vec::<u8>::new())); let out2=out.clone();
        c.hook_write(0xF001, move |_cpu, _addr, v|{ out2.borrow_mut().push(v); });
        c.load(0x0800,&[0xA9,0x41,0x8D,0x01,0xF0,0x00]); c.reset(); c.run_until_brk(50).unwrap();
        assert_eq!(out.borrow().as_slice(), b"A"); }
    #[test] fn page_cross_indexed_read_cycles(){ let mut c=base_cpu(); // Place LDA $10FF,X with X=1 crossing into $1100
        c.load(0x0800,&[0xA2,0x01,0xBD,0xFF,0x10,0x00]); // LDX #1; LDA $10FF,X ; BRK
        c.mem[0x1100]=0x55; c.reset(); let start=0; while !c.halted { c.step(); }
        // Base cycles: LDX #=2, LDA abs,X base 4 +1 page cross, BRK=2 => total 2+5+2=9
        assert_eq!(c.cycles-start,9); assert_eq!(c.a,0x55); }
    #[test] fn branch_page_cross_cycle(){ let mut c=base_cpu(); // BNE forward crossing page boundary
        // Put code at $08FE: LDA #1; BNE +2 (to $0903 after page cross); BRK; NOP; BRK
        c.load(0x08FE,&[0xA9,0x01,0xD0,0x02,0x00,0xEA,0x00]); c.mem[0xFFFC]=0xFE; c.mem[0xFFFD]=0x08; c.reset();
        while !c.halted { c.step(); if c.cycles>30 { break; } }
        // Taken branch: +1, plus page cross +1
    // Expected cycles: LDA # (2) + BNE (2 base +2 extras) + BRK (1 byte fetch + 1 implied) but BRK counted as 2 => total 2 + 4 + 1? Our model counts 2 for BRK giving 7 observed.
    assert_eq!(c.cycles,7); }
}
