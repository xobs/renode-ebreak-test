pub(crate) struct Serial;

use core::{arch::global_asm, u8};

extern "C" {
    static tohost: *mut u32;
    static fromhost: *mut u32;
}

// This value needs to be written to UART_BASE+4 in order
// to print a character.
const CONSOLE_OUTPUT: u32 = 0x0101_0000;

impl Serial {
    pub fn putc(&mut self, c: u8) {
        unsafe {
            let addr = (&tohost as *const _ as usize) as *mut u32;
            addr.write_volatile(c as u32);
            addr.add(1).write_volatile(CONSOLE_OUTPUT);
            while addr.read_volatile() != 0 {}
        }
    }
}

#[macro_export]
macro_rules! print {
    ($($args:tt)+) => {{
        #[allow(unused_unsafe)]
        unsafe {
            use core::fmt::Write;
            use crate::riscv_support::Serial;
            write!(Serial, $($args)+).unwrap();
        }
    }};
}

/// Prints to the debug output directly, with a newline.
#[macro_export]
macro_rules! println {
        () => ({
                print!("\r\n")
        });
        ($fmt:expr) => ({
                print!(concat!($fmt, "\r\n"))
        });
        ($fmt:expr, $($args:tt)+) => ({
                print!(concat!($fmt, "\r\n"), $($args)+)
        });
}

impl core::fmt::Write for Serial {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        for c in s.bytes() {
            self.putc(c);
        }
        Ok(())
    }
}

pub(crate) fn exit(_code: i32) -> ! {
    loop {
        unsafe {
            tohost.write_volatile(1);
            tohost.add(1).write_volatile(0);
            fromhost.read_volatile();
        }
    }
}

#[derive(Debug)]
enum Opcode {
    Opcode16(u16),
    Opcode32(u32),
}

impl core::fmt::LowerHex for Opcode {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Opcode::Opcode16(val) => write!(f, "{:04x}", val),
            Opcode::Opcode32(val) => write!(f, "{:08x}", val),
        }
    }
}

impl Opcode {
    fn read(addr: u32) -> Self {
        let addr = addr as *const u8;
        let start = unsafe { addr.read_volatile() };
        if start & 0b11 == 0b11 {
            let mut opcode_raw = [0u8; 4];
            opcode_raw[0] = start;
            opcode_raw[1] = unsafe { addr.add(1).read_volatile() };
            opcode_raw[2] = unsafe { addr.add(2).read_volatile() };
            opcode_raw[3] = unsafe { addr.add(3).read_volatile() };
            Self::Opcode32(u32::from_le_bytes(opcode_raw))
        } else {
            let mut opcode_raw = [0u8; 2];
            opcode_raw[0] = start;
            opcode_raw[1] = unsafe { addr.add(1).read_volatile() };
            Self::Opcode16(u16::from_le_bytes(opcode_raw))
        }
    }

    fn write(&self, addr: u32) {
        let addr = addr as *mut u8;
        match self {
            Opcode::Opcode16(val) => {
                let opcode_raw = val.to_le_bytes();
                unsafe { addr.write_volatile(opcode_raw[0]) };
                unsafe { addr.add(1).write_volatile(opcode_raw[1]) };
            }
            Opcode::Opcode32(val) => {
                let opcode_raw = val.to_le_bytes();
                unsafe { addr.write_volatile(opcode_raw[0]) };
                unsafe { addr.add(1).write_volatile(opcode_raw[1]) };
                unsafe { addr.add(2).write_volatile(opcode_raw[2]) };
                unsafe { addr.add(3).write_volatile(opcode_raw[3]) };
            }
        }
    }

    fn patch(&self, addr: u32) {
        match self {
            Opcode::Opcode16(_) => Opcode::Opcode16(0x9002).write(addr),
            Opcode::Opcode32(_) => Opcode::Opcode32(0x0010_0073).write(addr),
        }
    }

    fn next_pc(&self, pc: u32, registers: &[u32; 32]) -> u32 {
        match self {
            Opcode::Opcode16(val) => self.next_pc_16(pc, registers, *val as u32).unwrap(),
            Opcode::Opcode32(val) => self.next_pc_32(pc, registers, *val).unwrap(),
        }
    }

    fn next_pc_32(&self, pc: u32, registers: &[u32; 32], opcode: u32) -> Result<u32, ()> {
        // jal:  xxxxxxxxxxxxxxxxxxxxxxxxx1101111
        if opcode & 0b1111111 == 0b110_1111 {
            let mut imm = (opcode >> 20) & 0b0111_1111_1111;
            if opcode & 0x80000000 != 0 {
                imm |= 0xfff0_0000;
            }
            let rs1 = (opcode >> 15) & 0b11111;
            let rs1_val = registers[rs1 as usize];

            // println!(
            //     "jal {}(x{} [{:08x}]) [{:08x}]",
            //     imm as isize,
            //     rs1,
            //     rs1_val,
            //     rs1_val.wrapping_add(imm)
            // );
            Ok(rs1_val.wrapping_add(imm))
        }
        // jalr: xxxxxxxxxxxxxxxxx000xxxxx1100111
        else if opcode & 0b000000000000_00000_111_00000_1111111
            == 0b000000000000_00000_000_00000_1100111
        {
            let mut imm = (opcode >> 20) & 0b111_1111_1111;
            if opcode & 0x80000000 != 0 {
                imm |= 0xffff_f800;
            }

            let rs1 = (opcode >> 15) & 0b11111;
            let rs1_val = registers[rs1 as usize];

            // println!(
            //     "jal ??, x{} [{:08x}], {}  # {:08x}",
            //     rs1,
            //     rs1_val,
            //     imm as isize,
            //     rs1_val.wrapping_add(imm)
            // );

            Ok(rs1_val.wrapping_add(imm))
        } else if opcode & 0b1111111 == 0b1100011 {
            let mut imm = ((opcode >> 7) & 0b0_000000_1111_0)
                | (opcode >> 20) & 0b0_111111_0000_0
                | (opcode << 4) & 0b1_000000_0000_0;
            if opcode & 0x80000000 != 0 {
                imm |= 0xffff_f000;
            }

            let rs1 = (opcode >> 15) & 0b11111;
            let rs1_val = registers[rs1 as usize];

            let rs2 = (opcode >> 20) & 0b11111;
            let rs2_val = registers[rs2 as usize];

            Ok(pc.wrapping_add(match (opcode >> 12) & 0b111 {
                // beq
                0b000 => {
                    if rs1_val == rs2_val {
                        // println!(
                        //     "beq x{} [{:08x}], x{} [{:08x}], {} [{:08x}]",
                        //     rs1,
                        //     rs1_val,
                        //     rs2,
                        //     rs2_val,
                        //     imm as isize,
                        //     pc.wrapping_add(imm)
                        // );
                        imm
                    } else {
                        // println!(
                        //     "beq x{} [{:08x}], x{} [{:08x}], 4 [{:08x}]",
                        //     rs1,
                        //     rs1_val,
                        //     rs2,
                        //     rs2_val,
                        //     pc.wrapping_add(4)
                        // );
                        4
                    }
                }
                // bne
                0b001 => {
                    if rs1_val != rs2_val {
                        // println!(
                        //     "bne x{} [{:08x}], x{} [{:08x}], {} [{:08x}]",
                        //     rs1,
                        //     rs1_val,
                        //     rs2,
                        //     rs2_val,
                        //     imm as isize,
                        //     pc.wrapping_add(imm)
                        // );
                        imm
                    } else {
                        // println!(
                        //     "bne x{} [{:08x}], x{} [{:08x}], 4 [{:08x}]",
                        //     rs1,
                        //     rs1_val,
                        //     rs2,
                        //     rs2_val,
                        //     pc.wrapping_add(4)
                        // );
                        4
                    }
                }
                // blt
                0b100 => {
                    if rs1_val < rs2_val {
                        // println!(
                        //     "blt x{} [{:08x}], x{} [{:08x}], {} [{:08x}]",
                        //     rs1,
                        //     rs1_val,
                        //     rs2,
                        //     rs2_val,
                        //     imm as isize,
                        //     pc.wrapping_add(imm)
                        // );
                        imm
                    } else {
                        // println!(
                        //     "blt x{} [{:08x}], x{} [{:08x}], 4 [{:08x}]",
                        //     rs1,
                        //     rs1_val,
                        //     rs2,
                        //     rs2_val,
                        //     pc.wrapping_add(4)
                        // );
                        4
                    }
                }
                // bge
                0b101 => {
                    if rs1_val >= rs2_val {
                        // println!(
                        //     "bge x{} [{:08x}], x{} [{:08x}], {} [{:08x}]",
                        //     rs1,
                        //     rs1_val,
                        //     rs2,
                        //     rs2_val,
                        //     imm as isize,
                        //     pc.wrapping_add(imm)
                        // );
                        imm
                    } else {
                        // println!(
                        //     "bge x{} [{:08x}], x{} [{:08x}], 4 [{:08x}]",
                        //     rs1,
                        //     rs1_val,
                        //     rs2,
                        //     rs2_val,
                        //     pc.wrapping_add(4)
                        // );
                        4
                    }
                }
                // bltu
                0b110 => {
                    if rs1_val < rs2_val {
                        // println!(
                        //     "bltu x{} [{:08x}], x{} [{:08x}], {} [{:08x}]",
                        //     rs1,
                        //     rs1_val,
                        //     rs2,
                        //     rs2_val,
                        //     imm as isize,
                        //     pc.wrapping_add(imm)
                        // );
                        imm
                    } else {
                        // println!(
                        //     "bltu x{} [{:08x}], x{} [{:08x}], 4 [{:08x}]",
                        //     rs1,
                        //     rs1_val,
                        //     rs2,
                        //     rs2_val,
                        //     pc.wrapping_add(4)
                        // );
                        4
                    }
                }
                // bgeu
                0b111 => {
                    if rs1_val >= rs2_val {
                        // println!(
                        //     "bgeu x{} [{:08x}], x{} [{:08x}], {} [{:08x}]",
                        //     rs1,
                        //     rs1_val,
                        //     rs2,
                        //     rs2_val,
                        //     imm as isize,
                        //     pc.wrapping_add(imm)
                        // );
                        imm
                    } else {
                        // println!(
                        //     "bgeu x{} [{:08x}], x{} [{:08x}], 4 [{:08x}]",
                        //     rs1,
                        //     rs1_val,
                        //     rs2,
                        //     rs2_val,
                        //     pc.wrapping_add(4)
                        // );
                        4
                    }
                }
                _ => pc + 4,
            }))
        } else {
            // println!("other {:08x}", opcode);
            Ok(pc.wrapping_add(4))
        }
    }

    fn next_pc_16(&self, pc: u32, registers: &[u32; 32], opcode: u32) -> Result<u32, ()> {
        let opcode = opcode as u32;
        if opcode & 0b1110_00000_1111111 == 0b1000_00000_0000010 {
            // c.jr or c.jalr
            let rs1 = (opcode >> 7) & 0b11111;
            let rs1_val = registers[rs1 as usize];
            // if opcode & 0b0001_0000_0000_0000 == 0b0001_0000_0000_0000 {
            //     println!("c.jalr x{} [{:08x}]", rs1, rs1_val);
            // } else {
            //     println!("c.jr x{} [{:08x}]", rs1, rs1_val);
            // }
            Ok(rs1_val)
        } else if opcode & 0b011_0000000000011 == 0b001_00000000000_01 {
            // c.j  or c.jal
            // [11|4|9:8|10|6|7|3:1|5]
            let mut imm = (((opcode >> (3 - 1)) & 0b00000001110)
                | (opcode >> (11 - 4)) & 0b00000010000
                | (opcode << (5 - 2)) & 0b00000100000
                | (opcode >> (7 - 6)) & 0b00001000000
                | (opcode << (7 - 6)) & 0b00010000000
                | (opcode >> (9 - 8)) & 0b01100000000
                | (opcode << (10 - 8)) & 0b10000000000) as u32;
            // Sign extend
            if opcode & 0b0001_0000_0000_0000 != 0 {
                imm |= 0xffff_f800;
            }
            // if opcode & 0b1000_0000_0000_0000 == 0b1000_0000_0000_0000 {
            //     println!("c.j {}", pc.wrapping_add(imm));
            // } else {
            //     println!("c.jal {}", pc.wrapping_add(imm));
            // }

            Ok(pc.wrapping_add(imm))
        } else if opcode & 0b110_00000000000_11 == 0b110_00000000000_01 {
            // c.bnez  or c.beqz
            let rs1 = ((opcode >> 7) & 0b111) | 0b1000;
            let rs1_val = registers[rs1 as usize];

            let mut imm = (((opcode >> 2) & 0b00_0_00_11_0)
                | (opcode >> (10 - 3)) & 0b00_0_11_00_0
                | (opcode << (5 - 2)) & 0b00_1_00_00_0
                | (opcode << (7 - 6)) & 0b11_0_00_00_0) as u32;
            if opcode & (1 << 12) != 0 {
                imm |= 0xffff_ff00;
            }

            if opcode & 0b001_00000000000_00 == 0b001_00000000000_00 {
                let target = if rs1_val != 0 {
                    pc.wrapping_add(imm)
                } else {
                    pc + 2
                };
                // println!(
                //     "c.bnez x{} [{:08x}], {} # {:08x}",
                //     rs1, rs1_val, imm as i32, target
                // );
                Ok(target)
            } else {
                let target = if rs1_val == 0 {
                    pc.wrapping_add(imm)
                } else {
                    pc + 2
                };
                // println!(
                //     "c.beqz x{} [{:08x}], {} # {:08x}",
                //     rs1, rs1_val, imm as i32, target
                // );
                Ok(target)
            }
        } else {
            // println!("c.other {:04x}", opcode as u16);
            Ok(pc.wrapping_add(2))
        }
    }

    pub fn size(&self) -> usize {
        match self {
            Opcode::Opcode16(_) => 2,
            Opcode::Opcode32(_) => 4,
        }
    }
}

fn epc_read() -> u32 {
    let existing: u32;
    unsafe { core::arch::asm!("csrr {0}, mepc", out(reg) existing) };
    existing
}

global_asm!(
    "
.global _start_trap
        
_start_trap:
    addi sp, sp, -32*32

    sw x0, 0*4(sp)
    sw x1, 1*4(sp)
    sw x2, 2*4(sp)
    sw x3, 3*4(sp)
    sw x4, 4*4(sp)
    sw x5, 5*4(sp)
    sw x6, 6*4(sp)
    sw x7, 7*4(sp)
    sw x8, 8*4(sp)
    sw x9, 9*4(sp)
    sw x10, 10*4(sp)
    sw x11, 11*4(sp)
    sw x12, 12*4(sp)
    sw x13, 13*4(sp)
    sw x14, 14*4(sp)
    sw x15, 15*4(sp)
    sw x16, 16*4(sp)
    sw x17, 17*4(sp)
    sw x18, 18*4(sp)
    sw x19, 19*4(sp)
    sw x20, 20*4(sp)
    sw x21, 21*4(sp)
    sw x22, 22*4(sp)
    sw x23, 23*4(sp)
    sw x24, 24*4(sp)
    sw x25, 25*4(sp)
    sw x26, 26*4(sp)
    sw x27, 27*4(sp)
    sw x28, 28*4(sp)
    sw x29, 29*4(sp)
    sw x30, 30*4(sp)
    sw x31, 31*4(sp)

    add a0, sp, zero
    jal ra, replace_ebreak

    lw x0, 0*4(sp)
    lw x1, 1*4(sp)
    lw x2, 2*4(sp)
    lw x3, 3*4(sp)
    lw x4, 4*4(sp)
    lw x5, 5*4(sp)
    lw x6, 6*4(sp)
    lw x7, 7*4(sp)
    lw x8, 8*4(sp)
    lw x9, 9*4(sp)
    lw x10, 10*4(sp)
    lw x11, 11*4(sp)
    lw x12, 12*4(sp)
    lw x13, 13*4(sp)
    lw x14, 14*4(sp)
    lw x15, 15*4(sp)
    lw x16, 16*4(sp)
    lw x17, 17*4(sp)
    lw x18, 18*4(sp)
    lw x19, 19*4(sp)
    lw x20, 20*4(sp)
    lw x21, 21*4(sp)
    lw x22, 22*4(sp)
    lw x23, 23*4(sp)
    lw x24, 24*4(sp)
    lw x25, 25*4(sp)
    lw x26, 26*4(sp)
    lw x27, 27*4(sp)
    lw x28, 28*4(sp)
    lw x29, 29*4(sp)
    lw x30, 30*4(sp)
    lw x31, 31*4(sp)

    addi sp, sp, 32*32

    mret
"
);

static mut PATCHED_OPCODE: Option<Opcode> = None;
#[export_name = "replace_ebreak"]
extern "C" fn replace_ebreak(trap_frame: &[u32; 32]) {
    // The very first trap instruction is a `c.ebreak`, so replace
    // it with `c.nop`.
    let current_opcode = unsafe { PATCHED_OPCODE.take().unwrap_or(Opcode::Opcode16(0x0001)) };

    let current_pc = epc_read();
    // Unpatch the current opcode
    current_opcode.write(current_pc);

    // Figure out the next address to be executed
    let new_pc = current_opcode.next_pc(current_pc, trap_frame);
    let new_opcode = Opcode::read(new_pc);
    println!(
        "{}-bit Opcode at PC {:08x} {:x} -> {}-bit opcode at PC {:08x}: {:08x}",
        current_opcode.size() * 8,
        current_pc,
        current_opcode,
        new_opcode.size() * 8,
        new_pc,
        new_opcode
    );

    new_opcode.patch(new_pc);
    unsafe { PATCHED_OPCODE = Some(new_opcode) };
}
