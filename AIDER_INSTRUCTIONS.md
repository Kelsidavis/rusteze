# RustOS - Unix-like Operating System Roadmap

## 🎯 Vision & Goals

**RustOS** is evolving into a capable, demo-worthy hobby OS that showcases modern kernel development in Rust. Our journey:

- **Foundation Complete**: Boot, interrupts, memory management, and hardware drivers ✓
- **Current Phase**: Process management and multitasking
- **Near Future**: Virtual filesystem, userspace programs, and a working shell
- **Ambitious Goals**: Networking stack, graphics modes, multi-core support, and a rich set of utilities

**What makes RustOS unique:**
- Pure Rust kernel with no legacy C code
- Clean architecture built from first principles
- Focus on understandability and educational value
- Practical features: networking, graphics, real filesystem support
- Goal: Boot to a functional shell that can run programs, manipulate files, and communicate over the network

---

## 🎉 Recent Achievements

**Hardware Foundation (COMPLETE)**:
- Full interrupt handling (IDT, PIC, exceptions)
- Timer-based preemption (PIT @ 100Hz)
- Memory management (physical allocator, paging, heap)
- Input devices (PS/2 keyboard + mouse)
- Storage (PCI enumeration, ATA/IDE disk driver in PIO mode)

**Process Infrastructure (IN PROGRESS)**:
- Process Control Block with register state
- Basic context switching scaffolding
- Round-robin scheduler structure

This is solid progress! The kernel has reached the critical milestone where it can manage hardware. Now it's time to bring it to life with multitasking and userspace.

---

## Phase 1: Boot & Core (Foundation) ✓
- [x] Bootloader entry point (no_std, no_main)
- [x] Basic kernel that compiles
- [x] VGA text mode driver (80x25, 16 colors)
- [x] println! macro for kernel output
- [x] Serial port driver (COM1) for debug output
- [x] Panic handler with message display

## Phase 2: CPU & Memory Setup ✓
- [x] Global Descriptor Table (GDT)
- [x] Task State Segment (TSS)
- [x] Interrupt Descriptor Table (IDT)
- [x] Exception handlers (page fault, double fault, etc.)
- [x] Programmable Interrupt Controller (PIC) setup
- [x] Timer interrupt (PIT) at 100Hz
- [x] Physical memory manager (bitmap allocator)
- [x] Virtual memory / paging (4-level page tables)
- [x] Kernel heap allocator (linked list)

## Phase 3: Hardware Abstraction ✓
- [x] PS/2 keyboard driver with scancode translation
- [x] PS/2 mouse driver
- [x] PCI bus enumeration
- [x] ATA/IDE disk driver (PIO mode)

## Phase 4: Process Management (IN PROGRESS)
**Goal**: Enable true multitasking with timer-based preemption

- [x] Process Control Block (PCB) structure (src/process.rs)
- [x] Process states enum (running, ready, blocked, zombie)
- [ ] Fix context switching assembly (current implementation has bugs)
  - Hint: Use proper naked functions or external ASM files
  - Save/restore ALL registers including r8-r15, flags
  - Handle stack switching correctly
- [ ] Integrate scheduler with timer interrupt (PIT)
  - On each timer tick, save current context and load next process
- [ ] Kernel threads (cooperative multitasking first)
  - Create simple kernel tasks to test scheduling
  - Example: idle task, background task printing dots
- [ ] Process/thread creation API (spawn_kernel_thread)
- [ ] Process termination and cleanup (exit, reaping zombies)

## Phase 5: System Calls & User Mode
**Goal**: Ring 0 → Ring 3 transition, syscall interface

- [ ] User mode (Ring 3) support
  - Update GDT with user code/data segments
  - Implement privilege level switching
- [ ] Syscall dispatcher (int 0x80 or syscall/sysret instruction)
  - Handler that saves user context, validates syscall number
  - Dispatch table mapping syscall numbers to kernel functions
- [ ] Basic syscalls (start with 5-10 core syscalls):
  - `write(fd, buf, len)` - output to screen/serial
  - `read(fd, buf, len)` - input from keyboard
  - `exit(code)` - terminate process
  - `getpid()` - get process ID
  - `fork()` - create child process (advanced)
  - `exec(path)` - replace process with new program
- [ ] User stack setup (separate from kernel stack)
- [ ] Return to userspace (iretq with correct stack frame)

## Phase 6: Virtual Filesystem (VFS)
**Goal**: Unified file interface for RAM, disk, and devices

- [ ] VFS layer with inode abstraction
  - Define Inode, File, Dentry structures
  - Virtual operations: open, close, read, write, seek
  - Mount points and filesystem registration
- [ ] File descriptor table (per process, array of File*)
  - stdin (fd 0), stdout (fd 1), stderr (fd 2)
  - open() returns fd, read/write use fd
- [ ] tmpfs (in-memory filesystem)
  - Create files in RAM using heap allocator
  - Directory tree traversal (path parsing)
  - Implement open, read, write, mkdir, ls
- [ ] devfs (/dev with device nodes)
  - /dev/null, /dev/zero
  - /dev/tty (console)
  - /dev/hda (ATA disk)
- [ ] procfs (/proc for process info)
  - /proc/meminfo (memory stats)
  - /proc/<pid>/status (process state)

## Phase 7: User Space Programs
**Goal**: Load and run actual programs, boot to shell

- [ ] ELF binary loader
  - Parse ELF headers (check magic, architecture)
  - Load program segments into memory
  - Set up entry point, stack, and jump to user mode
- [ ] Static binaries first (no dynamic linking)
  - Embedded ELF files in kernel image for testing
  - Or load from initramfs in memory
- [ ] Init process (PID 1)
  - First userspace process spawned by kernel
  - Responsible for launching shell
- [ ] Basic shell (`/bin/sh`) with commands:
  - `echo <text>` - print text
  - `clear` - clear screen
  - `help` - show available commands
  - `ps` - list processes
  - `cat <file>` - display file contents
  - `ls [dir]` - list directory
  - `exit` - terminate shell (causes kernel panic for now)
  - `reboot` - reboot system
- [ ] Command parsing and execution loop
  - Read line from stdin
  - Parse into command + args
  - Fork/exec or builtin dispatch

## Phase 8: Storage & Real Filesystem
**Goal**: Persistent storage with a real filesystem

- [ ] MBR/GPT partition table parsing
  - Read partition entries from disk sector 0
  - Identify filesystem types
- [ ] FAT32 filesystem driver (simple, well-documented)
  - Read FAT tables and directory entries
  - File read/write operations
  - Create files and directories
- [ ] Or ext2 filesystem driver (Linux-native, also simple)
  - Superblock, block groups, inodes
  - Read/write files via block layer
- [ ] Disk caching/buffering
  - Cache frequently-used disk sectors in RAM
  - Write-through or write-back strategy
- [ ] Mount real filesystem on boot
  - Mount root filesystem from /dev/hda1
  - Populate /dev, /proc as virtual filesystems

## Phase 9: Advanced Graphics
**Goal**: Move beyond text mode, support graphical output

- [ ] VESA/VBE framebuffer driver
  - Query available video modes
  - Set a graphics mode (e.g., 1024x768x32)
  - Linear framebuffer access
- [ ] Pixel plotting and basic drawing primitives
  - Draw pixel, line, rectangle, circle
  - Fill operations
- [ ] Bitmap font rendering
  - Embedded PSF or BDF font
  - Render text in graphics mode
- [ ] Simple framebuffer console
  - Scrolling text output in graphics mode
  - Cursor rendering
- [ ] Mouse cursor rendering
  - Draw cursor sprite at mouse position
  - Handle mouse movement from PS/2 driver

## Phase 10: Networking
**Goal**: Connect to the network, implement TCP/IP

- [ ] Network device driver (start with RTL8139 or E1000)
  - PCI device detection and initialization
  - Send/receive packet buffers
  - IRQ handling for packet arrival
- [ ] Ethernet frame parsing
  - Parse MAC addresses, EtherType
  - ARP protocol (IP ↔ MAC resolution)
- [ ] IP layer (IPv4)
  - Parse IP headers (source, dest, protocol)
  - IP routing (basic forwarding)
  - ICMP (ping request/reply)
- [ ] UDP protocol
  - Datagram send/receive
  - Port-based demultiplexing
- [ ] TCP protocol (challenging but rewarding!)
  - Connection establishment (SYN, SYN-ACK, ACK)
  - Reliable transmission (sequence numbers, ACKs, retransmission)
  - Flow control (window management)
  - Connection teardown (FIN)
- [ ] Socket API (POSIX-like)
  - socket(), bind(), listen(), accept(), connect()
  - send(), recv(), sendto(), recvfrom()
  - close(), shutdown()
- [ ] DHCP client (auto-configure IP address)
- [ ] DNS client (resolve domain names)
- [ ] Simple HTTP client or server
  - Fetch web pages or serve static files
  - Demo: Download a file from the internet!

## Phase 11: Multi-Core & Advanced Scheduling
**Goal**: Utilize multiple CPU cores, improve performance

- [ ] SMP (Symmetric Multi-Processing) initialization
  - Detect number of CPU cores (ACPI tables or CPUID)
  - Wake up Application Processors (APs) via APIC
- [ ] Local APIC setup (per-core interrupts)
  - Replace PIC with APIC for better IRQ handling
  - Per-core timer interrupts
- [ ] Per-core scheduler and run queues
  - Each CPU has its own ready queue
  - Load balancing between cores
- [ ] Spinlocks for kernel synchronization
  - Protect shared data structures
  - Test-and-set atomic operations
- [ ] Mutex and semaphore primitives
- [ ] Improve scheduler (CFS-like or priority-based)

## Phase 12: Polished User Experience
**Goal**: Make RustOS fun to use and demo

- [ ] More shell utilities:
  - `mkdir`, `rmdir`, `rm`, `cp`, `mv`
  - `grep`, `wc`, `head`, `tail`
  - `date`, `uptime`, `free`
  - `kill` (send signals to processes)
- [ ] Terminal emulator improvements
  - ANSI escape code support (colors, cursor movement)
  - Scrollback buffer
- [ ] Text editor (simple vi-like or ed-like)
  - Open, edit, save files from the shell
- [ ] Simple games or demos:
  - Snake or Tetris in text mode
  - Mandelbrot fractal renderer in graphics mode
  - Starfield or plasma effect
- [ ] Boot splash screen or logo
- [ ] Configuration files (/etc/fstab, /etc/passwd)
- [ ] Multiple virtual consoles (Alt+F1, Alt+F2, etc.)

---

## 🎯 Immediate Priorities (Next 5 Tasks)

1. **Fix context switching** - The current assembly in src/process.rs has issues. Use naked functions or separate .asm files.
2. **Timer-based preemptive multitasking** - Hook scheduler into PIT interrupt for automatic task switching.
3. **Kernel thread spawning** - Create API to spawn simple kernel tasks for testing.
4. **Ring 3 user mode** - Update GDT and implement privilege switching so we can run userspace code.
5. **Syscall interface** - Implement int 0x80 handler and basic write/exit syscalls.

Once these are done, RustOS will be able to run multiple concurrent tasks and execute user programs!

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
**Phase**: 4 - Process Management (multitasking foundation)
**Next Task**: Fix context switching assembly code
**Vision**: Building toward a functional shell with networking and graphics!
