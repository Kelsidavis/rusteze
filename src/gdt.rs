// Global Descriptor Table (GDT) and Task State Segment (TSS) for x86_64
use core::mem::size_of;

/// GDT Entry - 8 bytes each
#[derive(Clone, Copy)]
#[repr(C, packed)]
pub struct GdtEntry {
    limit_low: u16,
    base_low: u16,
    base_middle: u8,
    access: u8,
    granularity: u8,
    base_high: u8,
}

impl GdtEntry {
    pub const fn null() -> Self {
        GdtEntry {
            limit_low: 0,
            base_low: 0,
            base_middle: 0,
            access: 0,
            granularity: 0,
            base_high: 0,
        }
    }

    pub const fn new(base: u32, limit: u32, access: u8, granularity: u8) -> Self {
        GdtEntry {
            limit_low: (limit & 0xFFFF) as u16,
            base_low: (base & 0xFFFF) as u16,
            base_middle: ((base >> 16) & 0xFF) as u8,
            access,
            granularity: ((limit >> 16) & 0x0F) as u8 | (granularity & 0xF0),
            base_high: ((base >> 24) & 0xFF) as u8,
        }
    }
}

/// System Segment Descriptor (16 bytes) - used for TSS in long mode
#[derive(Clone, Copy)]
#[repr(C, packed)]
pub struct SystemSegmentDescriptor {
    limit_low: u16,
    base_low: u16,
    base_middle: u8,
    access: u8,
    granularity: u8,
    base_high: u8,
    base_upper: u32,
    reserved: u32,
}

impl SystemSegmentDescriptor {
    pub const fn null() -> Self {
        SystemSegmentDescriptor {
            limit_low: 0,
            base_low: 0,
            base_middle: 0,
            access: 0,
            granularity: 0,
            base_high: 0,
            base_upper: 0,
            reserved: 0,
        }
    }

    /// Create a TSS descriptor from a base address and limit
    pub fn new_tss(base: u64, limit: u32) -> Self {
        SystemSegmentDescriptor {
            limit_low: (limit & 0xFFFF) as u16,
            base_low: (base & 0xFFFF) as u16,
            base_middle: ((base >> 16) & 0xFF) as u8,
            // TSS Available: Present | Type 0x9 (64-bit TSS Available)
            access: 0x89,
            granularity: ((limit >> 16) & 0x0F) as u8,
            base_high: ((base >> 24) & 0xFF) as u8,
            base_upper: (base >> 32) as u32,
            reserved: 0,
        }
    }
}

/// Task State Segment (TSS) for x86_64
/// Used for hardware task switching support and interrupt stack switching
#[repr(C, packed)]
pub struct TaskStateSegment {
    reserved_1: u32,
    /// Stack pointers for privilege levels 0-2
    /// RSP0 is used when transitioning from Ring 3 to Ring 0
    pub rsp0: u64,
    pub rsp1: u64,
    pub rsp2: u64,
    reserved_2: u64,
    /// Interrupt Stack Table (IST) - 7 separate stacks for specific interrupts
    /// IST1-7 can be assigned to specific interrupt handlers in the IDT
    pub ist1: u64,
    pub ist2: u64,
    pub ist3: u64,
    pub ist4: u64,
    pub ist5: u64,
    pub ist6: u64,
    pub ist7: u64,
    reserved_3: u64,
    reserved_4: u16,
    /// I/O Map Base Address (offset to I/O permission bitmap)
    pub iomap_base: u16,
}

impl TaskStateSegment {
    pub const fn new() -> Self {
        TaskStateSegment {
            reserved_1: 0,
            rsp0: 0,
            rsp1: 0,
            rsp2: 0,
            reserved_2: 0,
            ist1: 0,
            ist2: 0,
            ist3: 0,
            ist4: 0,
            ist5: 0,
            ist6: 0,
            ist7: 0,
            reserved_3: 0,
            reserved_4: 0,
            iomap_base: size_of::<TaskStateSegment>() as u16,
        }
    }
}

/// GDT Pointer structure for lgdt instruction
#[repr(C, packed)]
pub struct GdtPointer {
    limit: u16,
    base: u64,
}

// Access byte flags
const PRESENT: u8 = 1 << 7;
const DPL_RING0: u8 = 0 << 5;
const DPL_RING3: u8 = 3 << 5;
const DESCRIPTOR: u8 = 1 << 4;
const EXECUTABLE: u8 = 1 << 3;
const READ_WRITE: u8 = 1 << 1;

// Granularity byte flags
const LONG_MODE: u8 = 1 << 5;  // 64-bit code segment
const SIZE_32: u8 = 1 << 6;    // 32-bit protected mode
const GRANULARITY_4K: u8 = 1 << 7;

// Kernel code segment: Present, Ring 0, Executable, Readable, Long mode
const KERNEL_CODE_ACCESS: u8 = PRESENT | DPL_RING0 | DESCRIPTOR | EXECUTABLE | READ_WRITE;
const KERNEL_CODE_FLAGS: u8 = LONG_MODE | GRANULARITY_4K;

// Kernel data segment: Present, Ring 0, Writable
const KERNEL_DATA_ACCESS: u8 = PRESENT | DPL_RING0 | DESCRIPTOR | READ_WRITE;
const KERNEL_DATA_FLAGS: u8 = SIZE_32 | GRANULARITY_4K;

// User code segment: Present, Ring 3, Executable, Readable, Long mode
const USER_CODE_ACCESS: u8 = PRESENT | DPL_RING3 | DESCRIPTOR | EXECUTABLE | READ_WRITE;
const USER_CODE_FLAGS: u8 = LONG_MODE | GRANULARITY_4K;

// User data segment: Present, Ring 3, Writable
const USER_DATA_ACCESS: u8 = PRESENT | DPL_RING3 | DESCRIPTOR | READ_WRITE;
const USER_DATA_FLAGS: u8 = SIZE_32 | GRANULARITY_4K;

/// Stack size for interrupt stacks (16 KB each)
pub const INTERRUPT_STACK_SIZE: usize = 4096 * 4;

/// Double fault stack (IST index 0, but stored at IST1)
#[allow(dead_code)]
pub const DOUBLE_FAULT_IST_INDEX: u16 = 0;

/// Static storage for interrupt stacks
/// We allocate stacks for IST1 (double fault) initially
#[repr(C, align(16))]
struct InterruptStack {
    data: [u8; INTERRUPT_STACK_SIZE],
}

static mut DOUBLE_FAULT_STACK: InterruptStack = InterruptStack {
    data: [0; INTERRUPT_STACK_SIZE],
};

/// The Task State Segment
static mut TSS: TaskStateSegment = TaskStateSegment::new();

/// The GDT with entries:
/// 0: Null descriptor (required)
/// 1: Kernel code segment (0x08)
/// 2: Kernel data segment (0x10)
/// 3: User code segment (0x18)
/// 4: User data segment (0x20)
/// 5-6: TSS descriptor (0x28) - takes 16 bytes (2 entries)
#[repr(C, align(16))]
struct Gdt {
    null: GdtEntry,
    kernel_code: GdtEntry,
    kernel_data: GdtEntry,
    user_code: GdtEntry,
    user_data: GdtEntry,
    tss: SystemSegmentDescriptor,
}

static mut GDT: Gdt = Gdt {
    null: GdtEntry::null(),
    kernel_code: GdtEntry::new(0, 0xFFFFF, KERNEL_CODE_ACCESS, KERNEL_CODE_FLAGS),
    kernel_data: GdtEntry::new(0, 0xFFFFF, KERNEL_DATA_ACCESS, KERNEL_DATA_FLAGS),
    user_code: GdtEntry::new(0, 0xFFFFF, USER_CODE_ACCESS, USER_CODE_FLAGS),
    user_data: GdtEntry::new(0, 0xFFFFF, USER_DATA_ACCESS, USER_DATA_FLAGS),
    tss: SystemSegmentDescriptor::null(), // Will be initialized at runtime
};

static mut GDT_PTR: GdtPointer = GdtPointer { limit: 0, base: 0 };

/// Initialize and load the GDT with TSS
///
/// # Safety
/// This function must only be called once during kernel initialization.
/// It modifies global state and executes privileged CPU instructions.
pub unsafe fn init_gdt() {
    // Set up the TSS with interrupt stacks
    let double_fault_stack_end = (&raw const DOUBLE_FAULT_STACK as *const InterruptStack as u64)
        + INTERRUPT_STACK_SIZE as u64;

    TSS.ist1 = double_fault_stack_end; // IST1 for double fault handler

    // Create TSS descriptor
    let tss_base = &raw const TSS as u64;
    let tss_limit = (size_of::<TaskStateSegment>() - 1) as u32;
    GDT.tss = SystemSegmentDescriptor::new_tss(tss_base, tss_limit);

    // Set up the GDT pointer
    GDT_PTR.limit = (size_of::<Gdt>() - 1) as u16;
    GDT_PTR.base = &raw const GDT as u64;

    // Load the GDT
    core::arch::asm!(
        "lgdt [{}]",
        in(reg) &raw const GDT_PTR,
        options(nostack, preserves_flags)
    );

    // Reload segment registers
    // Code segment must be reloaded with a far jump
    core::arch::asm!(
        "push 0x08",           // Kernel code segment selector
        "lea rax, [rip + 2f]", // Address of label 2
        "push rax",
        "retfq",               // Far return to reload CS
        "2:",
        "mov ax, 0x10",        // Kernel data segment selector
        "mov ds, ax",
        "mov es, ax",
        "mov fs, ax",
        "mov gs, ax",
        "mov ss, ax",
        options(nostack)
    );

    // Load the TSS
    core::arch::asm!(
        "ltr ax",
        in("ax") TSS_SELECTOR,
        options(nostack, preserves_flags)
    );
}

/// Set the kernel stack pointer (RSP0) in the TSS
/// This is called when switching to a new process to set the kernel stack
/// that will be used when an interrupt occurs in user mode.
///
/// # Safety
/// Must be called with a valid stack pointer that has enough space.
#[allow(dead_code)]
pub unsafe fn set_kernel_stack(stack_ptr: u64) {
    TSS.rsp0 = stack_ptr;
}

/// Segment selectors for use elsewhere in the kernel
#[allow(dead_code)]
pub const KERNEL_CODE_SELECTOR: u16 = 0x08;
#[allow(dead_code)]
pub const KERNEL_DATA_SELECTOR: u16 = 0x10;
#[allow(dead_code)]
pub const USER_CODE_SELECTOR: u16 = 0x18 | 3; // Ring 3
#[allow(dead_code)]
pub const USER_DATA_SELECTOR: u16 = 0x20 | 3; // Ring 3
pub const TSS_SELECTOR: u16 = 0x28;
