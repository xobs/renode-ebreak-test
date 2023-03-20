#![cfg_attr(target_arch = "riscv32", no_std)]
#![cfg_attr(target_arch = "riscv32", no_main)]

#[cfg(target_arch = "riscv32")]
extern crate panic_halt;

#[cfg(target_arch = "riscv32")]
use riscv_rt::{entry, pre_init};

#[cfg_attr(target_arch = "riscv32", macro_use)]
#[cfg(target_arch = "riscv32")]
mod riscv_support;

#[cfg(target_arch = "riscv32")]
use riscv_support::exit;

#[cfg(not(target_arch = "riscv32"))]
use std::process::exit;

mod tests;

#[cfg(target_arch = "riscv32")]
#[pre_init]
unsafe fn add_hook() {}

#[cfg_attr(target_arch = "riscv32", entry)]
fn main() -> ! {
    println!("Starting up...");
    unsafe { core::arch::asm!("c.ebreak") };

    for n in 1..94 {
    // for n in 1..4 {
        println!("Fibonnaci of {}: {}", n, tests::fibonacci(n));
    }
    // do something here
    exit(0);
}
