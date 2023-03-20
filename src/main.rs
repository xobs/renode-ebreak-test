#![cfg_attr(target_arch = "riscv32", no_std)]
#![cfg_attr(target_arch = "riscv32", no_main)]

#[cfg(target_arch = "riscv32")]
extern crate panic_halt;

// Pull in symbols such as "_setup_interrupts", "__pre_init", etc.
#[cfg(target_arch = "riscv32")]
extern crate riscv_rt;

#[cfg(target_arch = "riscv32")]
use riscv_rt::entry;

#[cfg_attr(target_arch = "riscv32", entry)]
fn main() -> ! {
    // do something here
    loop {}
}
