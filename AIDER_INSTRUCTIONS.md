# RustOS - Unix-like Operating System Roadmap

## 🎯 Vision & Goals

**RustOS** is becoming a feature-rich, demo-worthy hobby OS that showcases what's possible with modern Rust kernel development. Our journey:

- **Foundation Complete** ✓: Boot, interrupts, memory management, and hardware drivers (3,100+ LOC!)
- **Current Phase**: Process management and multitasking (the kernel is about to come alive!)
- **Near Future**: Virtual filesystem, userspace programs, and interactive shell
- **Exciting Horizons**: TCP/IP networking, graphics/GUI, audio, real-time features, and POSIX compatibility

**What makes RustOS unique:**
- Pure Rust kernel with memory safety guarantees and zero legacy C code
- Clean architecture built from first principles - understandable and hackable
- Rich feature set: networking, graphics, audio, multi-core support
- Practical goal: Boot to shell → browse files → connect to internet → play audio/video
- Educational value: Perfect reference for OS development in Rust

**The Long-Term Dream:**
- Boot to a graphical desktop environment with window manager
- Run complex userspace applications (text editors, games, network clients)
- Support multiple users with permissions and security
- Networking: TCP/IP stack good enough to browse the web or host services
- Multimedia: Play audio, display images/video
- Development environment: Compile and run Rust programs *within* RustOS!

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

## Phase 13: Audio Subsystem
**Goal**: Sound output for notifications, music, and games

- [ ] AC97 or Intel HDA audio driver
  - Detect audio controller via PCI
  - Initialize DMA buffers for audio output
  - Configure sample rate (44.1kHz standard)
- [ ] Audio mixing layer
  - Mix multiple audio streams
  - Volume control per stream and master
- [ ] WAV file playback
  - Parse WAV headers (PCM format)
  - Stream audio data to hardware
- [ ] PC speaker driver (beep for simple sounds)
  - Use PIT channel 2 for tone generation
- [ ] Sound effects API for userspace
  - syscalls: `audio_open()`, `audio_write()`, `audio_close()`
- [ ] Demo: Play startup sound on boot
- [ ] Demo: Music player program (`play song.wav`)

## Phase 14: Advanced Drivers & Hardware
**Goal**: Support more devices for richer functionality

- [ ] USB subsystem (UHCI/OHCI/EHCI/xHCI)
  - USB controller driver (start with UHCI - simpler)
  - USB device enumeration
  - Hub support
- [ ] USB mass storage (flash drives)
  - SCSI command set for USB drives
  - Hot-plug detection and mounting
- [ ] USB keyboard/mouse support
  - HID (Human Interface Device) protocol
  - Fallback when PS/2 unavailable
- [ ] Real-time clock (RTC/CMOS)
  - Read current date/time
  - Set system time
  - Persist time across reboots
- [ ] AHCI/SATA driver (modern disk interface)
  - Replace PIO mode ATA with DMA transfers
  - Much faster disk I/O
- [ ] NVMe driver (modern SSDs)
  - PCIe-based storage
  - Extremely high performance
- [ ] Serial console improvements
  - Full terminal emulation over serial
  - Useful for debugging and remote access

## Phase 15: Memory Management Enhancements
**Goal**: More sophisticated and efficient memory handling

- [ ] Slab allocator for kernel objects
  - Replace linked-list heap with slab caches
  - Faster allocation for common object sizes
  - Reduces fragmentation
- [ ] Copy-on-write (COW) for fork()
  - Don't copy memory immediately on fork
  - Mark pages read-only and copy on write
  - Huge performance improvement
- [ ] Demand paging and page faults
  - Only load pages when accessed
  - Page fault handler allocates on demand
- [ ] Swap space support
  - Write unused pages to disk
  - Reclaim when memory pressure is high
  - Allows running larger programs
- [ ] Memory-mapped files (mmap)
  - Map files directly into process address space
  - Efficient file I/O without read/write syscalls
- [ ] Huge pages (2MB/1GB pages)
  - Reduce TLB pressure for large allocations
  - Performance boost for memory-intensive apps
- [ ] NUMA awareness (for multi-socket systems)
  - Allocate memory on same node as CPU
  - Important for scalability

## Phase 16: Inter-Process Communication (IPC)
**Goal**: Let processes communicate and coordinate

- [ ] Pipes (anonymous and named/FIFO)
  - `pipe()` syscall creates fd pair
  - Write to one end, read from other
  - Shell pipelines: `cat file.txt | grep foo`
- [ ] Unix domain sockets
  - Socket-based IPC on local machine
  - Used by many Unix daemons
- [ ] Shared memory segments
  - Multiple processes map same physical memory
  - Fastest IPC method
  - Requires synchronization (semaphores)
- [ ] Message queues
  - Send/receive discrete messages
  - Queue-based communication
- [ ] Signals
  - SIGTERM, SIGKILL, SIGUSR1, etc.
  - `kill()` syscall to send signals
  - Signal handlers in userspace
- [ ] Event notification (epoll/kqueue-like)
  - Monitor multiple fds for events
  - Efficient I/O multiplexing
  - Critical for network servers

## Phase 17: Security & Permissions
**Goal**: Multi-user support and access control

- [ ] User and group IDs
  - Each process has UID/GID
  - Root (UID 0) vs normal users
- [ ] File permissions (rwxrwxrwx)
  - Owner, group, other permission bits
  - Check permissions on open/read/write
- [ ] Password authentication
  - /etc/passwd and /etc/shadow files
  - Hash passwords with bcrypt or argon2
- [ ] Login program
  - Prompt for username/password
  - Start shell with correct UID if authenticated
- [ ] Sudo mechanism
  - Allow unprivileged users to run commands as root
  - /etc/sudoers configuration
- [ ] Capability-based security (optional, advanced)
  - Fine-grained permissions beyond UID/GID
  - Inspired by modern systems like Fuchsia

## Phase 18: Graphical User Interface (GUI)
**Goal**: Window manager and graphical applications

- [ ] Window manager (compositing or stacking)
  - Manage window positions, sizes, z-order
  - Window decorations (title bar, close button)
  - Mouse-based window dragging/resizing
- [ ] Graphics library for userspace
  - Drawing primitives accessible via syscalls
  - Or shared framebuffer with coordination
- [ ] Simple GUI toolkit
  - Buttons, text boxes, labels, menus
  - Event-driven programming model
- [ ] Demo applications:
  - Calculator
  - Text editor with GUI
  - File manager (browse directories graphically)
  - Image viewer (BMP/PNG support)
- [ ] Desktop environment
  - Taskbar, application launcher
  - System tray for notifications
  - Background wallpaper
- [ ] Support for bitmap fonts and TrueType fonts
  - Font rendering with anti-aliasing

## Phase 19: Developer Tools & Self-Hosting
**Goal**: Develop software within RustOS itself

- [ ] Port Rust compiler (rustc) to RustOS
  - Cross-compile rustc for RustOS target
  - Or run rustc via emulation layer
- [ ] Implement `cargo` equivalent
  - Build tool for Rust projects
  - Dependency management
- [ ] Text-based IDE or advanced editor
  - Syntax highlighting
  - Code completion (LSP?)
- [ ] Debugger (gdb-like)
  - Set breakpoints, step through code
  - Inspect variables and memory
- [ ] Version control (git port)
  - Clone, commit, push/pull repos
  - Manage RustOS development within RustOS!
- [ ] Shell scripting language
  - Bash-compatible or custom scripting
  - Automate tasks and system configuration
- [ ] Package manager
  - Install/update/remove software packages
  - Dependency resolution
  - Repository of pre-built binaries

## Phase 20: Advanced Networking & Services
**Goal**: Production-quality network services

- [ ] IPv6 support
  - Parse IPv6 headers and addresses
  - ICMPv6, NDP (neighbor discovery)
- [ ] Network bridging and routing
  - Forward packets between interfaces
  - NAT (Network Address Translation)
- [ ] Firewall/packet filter
  - iptables-like rules
  - Drop/accept based on criteria
- [ ] TLS/SSL support
  - Encrypt network connections
  - HTTPS client and server
  - Port mbedtls or rustls library
- [ ] SSH server and client
  - Secure remote shell access
  - SCP for file transfer
- [ ] NFS or SMB client
  - Mount network filesystems
  - Access files on remote servers
- [ ] Web browser (ambitious!)
  - HTML parser and renderer
  - CSS layout engine
  - JavaScript interpreter (or skip for now)
  - Goal: Browse simple websites
- [ ] Multiplayer game demo
  - Network-based game using TCP/UDP
  - Shows off networking and graphics

## Phase 21: Performance & Optimization
**Goal**: Make RustOS fast and efficient

- [ ] Profiling infrastructure
  - Sample-based profiler (perf-like)
  - Identify hotspots in kernel and userspace
- [ ] Lazy TLB flushing
  - Don't flush TLB unnecessarily on context switch
  - Track TLB generation numbers
- [ ] RCU (Read-Copy-Update) synchronization
  - Lock-free reads for shared data
  - Scalable concurrency primitive
- [ ] Zero-copy networking
  - DMA directly to userspace buffers
  - Avoid copying data through kernel
- [ ] Async I/O (io_uring style)
  - Submit I/O operations without blocking
  - Efficient for high-throughput apps
- [ ] JIT compilation for eBPF
  - Allow userspace to inject kernel code safely
  - Packet filtering, tracing, monitoring
- [ ] Power management (ACPI S-states)
  - Suspend to RAM, hibernate
  - CPU frequency scaling (SpeedStep/Turbo)
  - Laptop battery life optimization

## Phase 22: Virtualization & Containers
**Goal**: Run RustOS as hypervisor or in containers

- [ ] KVM-like virtualization support
  - Use hardware virtualization (VT-x/AMD-V)
  - Run guest VMs inside RustOS
- [ ] Paravirtualization drivers
  - VirtIO for efficient I/O in VMs
  - Run RustOS as guest with better performance
- [ ] Container runtime (Docker-like)
  - Namespaces for process isolation
  - cgroups for resource limits
  - Overlay filesystem for layers
- [ ] Micro-VM support (Firecracker-style)
  - Extremely lightweight VMs
  - Fast boot times for serverless workloads

## Phase 23: Exotic & Fun Features
**Goal**: Unique features that make RustOS stand out

- [ ] WASM runtime
  - Run WebAssembly modules in userspace
  - Sandboxed execution environment
  - Could run web apps locally
- [ ] Live kernel patching
  - Update kernel code without reboot
  - Critical for long-running systems
- [ ] Time travel debugging
  - Record execution and replay
  - Step backwards through program history
- [ ] Distributed filesystem (plan9-like)
  - Every resource is a file
  - Mount remote filesystems transparently
- [ ] Microkernel architecture experiment
  - Move drivers to userspace
  - IPC-based communication
  - Compare with monolithic kernel performance
- [ ] Formal verification of critical paths
  - Prove correctness of scheduler, memory allocator
  - Use Rust's type system + external tools
- [ ] Run on exotic architectures
  - RISC-V port
  - ARM64/AArch64 port
  - Test portability of Rust kernel code

---

## 🎯 Immediate Priorities (Next 10 Tasks)

1. **Fix context switching** - The current assembly in src/process.rs has issues. Use naked functions or separate .asm files.
2. **Timer-based preemptive multitasking** - Hook scheduler into PIT interrupt for automatic task switching.
3. **Kernel thread spawning** - Create API to spawn simple kernel tasks for testing.
4. **Process termination and cleanup** - Implement exit() and zombie reaping so processes can finish cleanly.
5. **Ring 3 user mode** - Update GDT and implement privilege switching so we can run userspace code.
6. **Syscall interface** - Implement int 0x80 handler and dispatch table.
7. **Basic syscalls: write/read/exit/getpid** - Core syscalls needed for any program.
8. **User stack setup** - Separate kernel/user stacks with proper privilege levels.
9. **VFS layer foundation** - Inode abstraction and file operations structure.
10. **tmpfs implementation** - In-memory filesystem to test VFS without disk complexity.

Once these are done, RustOS will run multitasking userspace programs with file I/O! That's the critical milestone that unlocks everything else.

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
**Lines of Code**: 3,100+ lines of pure Rust kernel code!
**Completed Sessions**: 20 sessions, 22 tasks done

**The Vision is Expanding**:
- Short term: Functional shell with file I/O and multitasking
- Medium term: Networking stack (TCP/IP) and graphical environment
- Long term: Self-hosting development environment, multimedia support, virtualization
- Dream goal: A complete OS that's fun to use and impressive to demo!

**Why This Matters**:
RustOS is proving that Rust is an excellent choice for OS development. Memory safety without garbage collection, zero-cost abstractions, and fearless concurrency make it possible to build a sophisticated kernel that's both safe and performant. Every feature we add demonstrates another aspect of systems programming in Rust.
