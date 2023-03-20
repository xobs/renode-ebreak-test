#!/bin/sh
echo "Copying binary to my.bin..."
riscv64-unknown-elf-objcopy -O binary ../target/riscv32imac-unknown-none-elf/debug/renode-ebreak-test my.bin

echo "Running temu..."
../../tinyemu-2019-12-21/temu --ctrlc my.cfg
