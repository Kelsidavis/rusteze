# RustOS - Unix-like Operating System Roadmap

## Phase 1: Boot & Core (Foundation)
- [x] Bootloader entry point (no_std, no_main)
- [x] Basic kernel that compiles
- [x] VGA text mode driver (80x25, 16 colors)
- [x] println! macro for kernel output
- [x] Serial port driver (COM1) for debug output
- [x] Panic handler with message display

## Phase 2: CPU & Memory Setup
- [ ] Global Descriptor Table (GDT)
- [ ] Task State Segment (TSS)
- [ ] Interrupt Descriptor Table (IDT)
- [ ] Exception handlers (page fault, double fault, etc.)
- [ ] Programmable Interrupt Controller (PIC) setup
- [ ] Timer interrupt (PIT) at 100Hz
- [ ] Physical memory manager (bitmap allocator)
- [ ] Virtual memory / paging (4-level page tables)
- [ ] Kernel heap allocator (linked list)

## Phase 3: Hardware Abstraction
- [ ] PS/2 keyboard driver with scancode translation
- [ ] PS/2 mouse driver
- [ ] PCI bus enumeration
- [ ] ATA/IDE disk driver (PIO mode)

## Phase 4: Process Management
- [ ] Process Control Block (PCB) structure
- [ ] Kernel threads
- [ ] Context switching (save/restore registers)
- [ ] Round-robin scheduler
- [ ] Process states (running, ready, blocked, zombie)

## Phase 5: System Calls
- [ ] Syscall dispatcher (int 0x80 or syscall instruction)
- [ ] Basic syscalls: write, exit, getpid

## Phase 6: Virtual Filesystem (VFS)
- [ ] VFS layer with inode abstraction
- [ ] File descriptor table (per process)
- [ ] tmpfs (in-memory filesystem)
- [ ] devfs (/dev with device nodes)

## Phase 7: User Space
- [ ] User mode (Ring 3) support
- [ ] ELF binary loader
- [ ] Init process (PID 1)
- [ ] Basic shell with: echo, clear, help, ps, exit

## Phase 8: Networking (Future)
- [ ] Network device driver
- [ ] TCP/IP stack
- [ ] Socket API

---

## Build & Test Commands
```bash
# Build kernel (warnings are errors)
RUSTFLAGS="-D warnings" cargo build --release

# The kernel library is at:
# target/x86_64-unknown-none/release/librusteze.rlib
```

## Development Rules
1. ALWAYS run build with RUSTFLAGS="-D warnings" after changes
2. Fix ALL compiler errors AND warnings before marking done
3. Only mark [x] when code compiles WITHOUT warnings
4. Test thoroughly before moving to next feature
5. Commit after each working feature

## Current Status
Phase: 2
Next task: Global Descriptor Table (GDT)
