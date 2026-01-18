// Global Descriptor Table (GDT) for x86_64
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

/// The GDT with 5 entries:
/// 0: Null descriptor (required)
/// 1: Kernel code segment
/// 2: Kernel data segment
/// 3: User code segment
/// 4: User data segment
#[repr(C, align(16))]
struct Gdt {
    entries: [GdtEntry; 5],
}

static mut GDT: Gdt = Gdt {
    entries: [
        GdtEntry::null(),                                           // 0x00: Null
        GdtEntry::new(0, 0xFFFFF, KERNEL_CODE_ACCESS, KERNEL_CODE_FLAGS), // 0x08: Kernel code
        GdtEntry::new(0, 0xFFFFF, KERNEL_DATA_ACCESS, KERNEL_DATA_FLAGS), // 0x10: Kernel data
        GdtEntry::new(0, 0xFFFFF, USER_CODE_ACCESS, USER_CODE_FLAGS),     // 0x18: User code
        GdtEntry::new(0, 0xFFFFF, USER_DATA_ACCESS, USER_DATA_FLAGS),     // 0x20: User data
    ],
};

static mut GDT_PTR: GdtPointer = GdtPointer { limit: 0, base: 0 };

/// Initialize and load the GDT
///
/// # Safety
/// This function must only be called once during kernel initialization.
/// It modifies global state and executes privileged CPU instructions.
pub unsafe fn init_gdt() {
    // Set up the GDT pointer
    GDT_PTR.limit = (size_of::<Gdt>() - 1) as u16;
    GDT_PTR.base = (&raw const GDT as *const Gdt) as u64;

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
