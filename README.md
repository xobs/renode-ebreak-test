Test ebreak by constantly issuing that command

## Building

Install Rust, then run:

```
rustup target add riscv32imac-unknown-none-elf
cargo build --target riscv32imac-unknown-none-elf
```

## Testing with tinyemu

To test with Tinyemu, run `tests/my.sh`.

## Testing with Renode

To test with Renode, use the following script:

```
mach create
using sysbus
machine LoadPlatformDescriptionFromString "cpu: CPU.RiscV32  @ sysbus { cpuType: \"rv32gc\"; timeProvider: empty }"
machine LoadPlatformDescriptionFromString "mem: Memory.MappedMemory @ sysbus 0x80000000 { size: 0x100000 }"
machine LoadPlatformDescriptionFromString "uart: UART.LiteX_UART @ sysbus 0x40008000"
sysbus LoadELF @my.elf
showAnalyzer uart
start
```

## Testing with Spike

To test with Spike, apply the following patch:

```patch
diff --git a/fesvr/htif.cc b/fesvr/htif.cc
index 3f93f7b5..563c9594 100644
--- a/fesvr/htif.cc
+++ b/fesvr/htif.cc
@@ -158,7 +158,8 @@ void htif_t::load_program()
     tohost_addr = symbols["tohost"];
     fromhost_addr = symbols["fromhost"];
   } else {
-    fprintf(stderr, "warning: tohost and fromhost symbols not in ELF; can't communicate with target\n");
+    tohost_addr = 0x40008000;
+    fromhost_addr = 0x40008000;
   }

   // detect torture tests so we can print the memory signature at the end
@@ -262,8 +263,13 @@ int htif_t::run()
     uint64_t tohost;

     try {
-      if ((tohost = from_target(mem.read_uint64(tohost_addr))) != 0)
-        mem.write_uint64(tohost_addr, target_endian<uint64_t>::zero);
+      if ((tohost = from_target(mem.read_uint64(tohost_addr))) != 0) {
+        if ((tohost == 1) || ((tohost & 0xffffffff00000000) != 0)) {
+          mem.write_uint64(tohost_addr, target_endian<uint64_t>::zero);
+        } else {
+          tohost = 0;
+        }
+      }
     } catch (mem_trap_t& t) {
       bad_address("accessing tohost", t.get_tval());
     }
diff --git a/riscv/platform.h b/riscv/platform.h
index 2bafa68d..3bc7ff18 100644
--- a/riscv/platform.h
+++ b/riscv/platform.h
@@ -2,7 +2,7 @@
 #ifndef _RISCV_PLATFORM_H
 #define _RISCV_PLATFORM_H

-#define DEFAULT_RSTVEC     0x00001000
+#define DEFAULT_RSTVEC     0x80000000
 #define CLINT_BASE         0x02000000
 #define CLINT_SIZE         0x000c0000
 #define PLIC_BASE          0x0c000000
 ```

 Then run Spike with:

```
 spike \
     -m0x80000000:1048576,0x40008000:4096 \
     --disable-dtb \
     --isa=RV32IMAC target/riscv32imac-unknown-none-elf/debug/renode-ebreak-test
```
