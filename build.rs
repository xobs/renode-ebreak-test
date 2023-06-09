use std::env;
use std::fs;
use std::path::PathBuf;

fn main() {
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let target = env::var("TARGET").unwrap();

    if target.starts_with("riscv") {
        // Put the linker script somewhere the linker can find it.
        fs::write(out_dir.join("memory.x"), include_bytes!("memory.x")).unwrap();
        println!("cargo:rustc-link-search={}", out_dir.display());

        println!("cargo:rerun-if-changed=memory.x");
        println!("cargo:rerun-if-changed=build.rs");

        println!("cargo:rustc-link-arg=-Tmemory.x");
        println!("cargo:rustc-link-arg=-Tlink.x");
    }
}
