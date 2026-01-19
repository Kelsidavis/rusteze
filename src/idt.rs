use x86_64::structures::idt::{InterruptDescriptorTable, InterruptStackFrame, PageFaultErrorCode};
use lazy_static::lazy_static;

// PIC ports
const PIC1_COMMAND: u16 = 0x20;
const PIC1_DATA: u16 = 0x21;
const PIC2_COMMAND: u16 = 0xA0;
const PIC2_DATA: u16 = 0xA1;

// PIC commands
const PIC_EOI: u8 = 0x20;

// IRQ offsets (remapped to start at 32)
const PIC1_OFFSET: u8 = 32;
const PIC2_OFFSET: u8 = 40;

// Exception handlers

extern "x86-interrupt" fn divide_error_handler(stack_frame: InterruptStackFrame) {
    panic!("EXCEPTION: DIVIDE ERROR\n{:#?}", stack_frame);
}

extern "x86-interrupt" fn debug_handler(stack_frame: InterruptStackFrame) {
    crate::serial_println!("EXCEPTION: DEBUG\n{:#?}", stack_frame);
}

extern "x86-interrupt" fn non_maskable_interrupt_handler(stack_frame: InterruptStackFrame) {
    panic!("EXCEPTION: NON-MASKABLE INTERRUPT\n{:#?}", stack_frame);
}

extern "x86-interrupt" fn breakpoint_handler(stack_frame: InterruptStackFrame) {
    crate::serial_println!("EXCEPTION: BREAKPOINT\n{:#?}", stack_frame);
}

extern "x86-interrupt" fn overflow_handler(stack_frame: InterruptStackFrame) {
    panic!("EXCEPTION: OVERFLOW\n{:#?}", stack_frame);
}

extern "x86-interrupt" fn bound_range_exceeded_handler(stack_frame: InterruptStackFrame) {
    panic!("EXCEPTION: BOUND RANGE EXCEEDED\n{:#?}", stack_frame);
}

extern "x86-interrupt" fn invalid_opcode_handler(stack_frame: InterruptStackFrame) {
    panic!("EXCEPTION: INVALID OPCODE\n{:#?}", stack_frame);
}

extern "x86-interrupt" fn device_not_available_handler(stack_frame: InterruptStackFrame) {
    panic!("EXCEPTION: DEVICE NOT AVAILABLE\n{:#?}", stack_frame);
}

extern "x86-interrupt" fn double_fault_handler(
    stack_frame: InterruptStackFrame,
    _error_code: u64,
) -> ! {
    panic!("EXCEPTION: DOUBLE FAULT\n{:#?}", stack_frame);
}

extern "x86-interrupt" fn invalid_tss_handler(
    stack_frame: InterruptStackFrame,
    error_code: u64,
) {
    panic!("EXCEPTION: INVALID TSS\nError Code: {}\n{:#?}", error_code, stack_frame);
}

extern "x86-interrupt" fn segment_not_present_handler(
    stack_frame: InterruptStackFrame,
    error_code: u64,
) {
    panic!("EXCEPTION: SEGMENT NOT PRESENT\nError Code: {}\n{:#?}", error_code, stack_frame);
}

extern "x86-interrupt" fn stack_segment_fault_handler(
    stack_frame: InterruptStackFrame,
    error_code: u64,
) {
    panic!("EXCEPTION: STACK SEGMENT FAULT\nError Code: {}\n{:#?}", error_code, stack_frame);
}

extern "x86-interrupt" fn general_protection_fault_handler(
    stack_frame: InterruptStackFrame,
    error_code: u64,
) {
    panic!(
        "EXCEPTION: GENERAL PROTECTION FAULT\nError Code: {}\n{:#?}",
        error_code, stack_frame
    );
}

extern "x86-interrupt" fn page_fault_handler(
    stack_frame: InterruptStackFrame,
    error_code: PageFaultErrorCode,
) {
    use x86_64::registers::control::Cr2;

    panic!(
        "EXCEPTION: PAGE FAULT\nAccessed Address: {:?}\nError Code: {:?}\n{:#?}",
        Cr2::read(),
        error_code,
        stack_frame
    );
}

extern "x86-interrupt" fn x87_floating_point_handler(stack_frame: InterruptStackFrame) {
    panic!("EXCEPTION: x87 FLOATING POINT\n{:#?}", stack_frame);
}

extern "x86-interrupt" fn alignment_check_handler(
    stack_frame: InterruptStackFrame,
    error_code: u64,
) {
    panic!("EXCEPTION: ALIGNMENT CHECK\nError Code: {}\n{:#?}", error_code, stack_frame);
}

extern "x86-interrupt" fn machine_check_handler(stack_frame: InterruptStackFrame) -> ! {
    panic!("EXCEPTION: MACHINE CHECK\n{:#?}", stack_frame);
}

extern "x86-interrupt" fn simd_floating_point_handler(stack_frame: InterruptStackFrame) {
    panic!("EXCEPTION: SIMD FLOATING POINT\n{:#?}", stack_frame);
}

extern "x86-interrupt" fn virtualization_handler(stack_frame: InterruptStackFrame) {
    panic!("EXCEPTION: VIRTUALIZATION\n{:#?}", stack_frame);
}

// Hardware interrupt handlers (IRQs)

extern "x86-interrupt" fn timer_interrupt_handler(_stack_frame: InterruptStackFrame) {
    // Increment the tick counter
    crate::pit::tick();

    // Perform context switch to next process (if scheduler is initialized)
    // Note: This is called every timer tick (100Hz), enabling preemptive multitasking
    unsafe {
        crate::process::PROCESS_MANAGER.lock().schedule();
    }

    // Send EOI (End of Interrupt) to PIC1
    unsafe {
        x86_64::instructions::port::Port::<u8>::new(PIC1_COMMAND).write(PIC_EOI);
    }
}

extern "x86-interrupt" fn keyboard_interrupt_handler(_stack_frame: InterruptStackFrame) {
    // Use the keyboard driver to process the scancode
    crate::keyboard::keyboard_interrupt_handler();

    // Send EOI
    unsafe {
        x86_64::instructions::port::Port::<u8>::new(PIC1_COMMAND).write(PIC_EOI);
    }
}

extern "x86-interrupt" fn mouse_interrupt_handler(_stack_frame: InterruptStackFrame) {
    // Use the mouse driver to process the packet byte
    crate::ps2_mouse::mouse_interrupt_handler();

    // Send EOI to both PICs (IRQ12 is on the slave PIC)
    unsafe {
        x86_64::instructions::port::Port::<u8>::new(PIC2_COMMAND).write(PIC_EOI);
        x86_64::instructions::port::Port::<u8>::new(PIC1_COMMAND).write(PIC_EOI);
    }
}

// System call handler (int 0x80)
extern "x86-interrupt" fn syscall_handler(_stack_frame: InterruptStackFrame) {
    // In a real implementation, we need to extract registers from the stack frame
    // For now, we'll use inline assembly to get the register values
    let rax: u64;
    let rdi: u64;
    let rsi: u64;
    let rdx: u64;
    let r10: u64;
    let r8: u64;
    let r9: u64;

    unsafe {
        core::arch::asm!(
            "mov {}, rax",
            "mov {}, rdi",
            "mov {}, rsi",
            "mov {}, rdx",
            "mov {}, r10",
            "mov {}, r8",
            "mov {}, r9",
            out(reg) rax,
            out(reg) rdi,
            out(reg) rsi,
            out(reg) rdx,
            out(reg) r10,
            out(reg) r8,
            out(reg) r9,
        );
    }

    // Dispatch the system call
    let result = crate::syscall::dispatch_syscall(rax, rdi, rsi, rdx, r10, r8, r9);

    // Return result in rax
    match result {
        Ok(value) => {
            // Set rax in the stack frame to the return value
            unsafe {
                core::arch::asm!("mov rax, {}", in(reg) value);
            }
        }
        Err(_) => {
            // Return -1 on error (Unix convention)
            unsafe {
                core::arch::asm!("mov rax, {}", in(reg) -1i64 as u64);
            }
        }
    }
}

// Static IDT using lazy_static
lazy_static! {
    static ref IDT: InterruptDescriptorTable = {
        let mut idt = InterruptDescriptorTable::new();

        // CPU exceptions (0-31)
        idt.divide_error.set_handler_fn(divide_error_handler);
        idt.debug.set_handler_fn(debug_handler);
        idt.non_maskable_interrupt.set_handler_fn(non_maskable_interrupt_handler);
        idt.breakpoint.set_handler_fn(breakpoint_handler);
        idt.overflow.set_handler_fn(overflow_handler);
        idt.bound_range_exceeded.set_handler_fn(bound_range_exceeded_handler);
        idt.invalid_opcode.set_handler_fn(invalid_opcode_handler);
        idt.device_not_available.set_handler_fn(device_not_available_handler);
        unsafe {
            idt.double_fault
                .set_handler_fn(double_fault_handler)
                .set_stack_index(crate::gdt::DOUBLE_FAULT_IST_INDEX);
        }
        idt.invalid_tss.set_handler_fn(invalid_tss_handler);
        idt.segment_not_present.set_handler_fn(segment_not_present_handler);
        idt.stack_segment_fault.set_handler_fn(stack_segment_fault_handler);
        idt.general_protection_fault.set_handler_fn(general_protection_fault_handler);
        idt.page_fault.set_handler_fn(page_fault_handler);
        idt.x87_floating_point.set_handler_fn(x87_floating_point_handler);
        idt.alignment_check.set_handler_fn(alignment_check_handler);
        idt.machine_check.set_handler_fn(machine_check_handler);
        idt.simd_floating_point.set_handler_fn(simd_floating_point_handler);
        idt.virtualization.set_handler_fn(virtualization_handler);

        // Hardware interrupts (IRQs remapped to 32-47)
        idt[PIC1_OFFSET].set_handler_fn(timer_interrupt_handler);      // IRQ0 - Timer
        idt[PIC1_OFFSET + 1].set_handler_fn(keyboard_interrupt_handler); // IRQ1 - Keyboard
        idt[PIC2_OFFSET + 4].set_handler_fn(mouse_interrupt_handler);   // IRQ12 - Mouse

        // System call interrupt (int 0x80)
        // Set DPL to Ring 3 so user mode can invoke it
        idt[0x80].set_handler_fn(syscall_handler)
            .set_privilege_level(x86_64::PrivilegeLevel::Ring3);

        idt
    };
}

/// Initialize the IDT
pub fn init_idt() {
    IDT.load();
}

/// Initialize the 8259 PIC (Programmable Interrupt Controller)
/// Remaps IRQ 0-7 to interrupts 32-39 and IRQ 8-15 to interrupts 40-47
pub fn init_pic() {
    unsafe {
        // Start initialization sequence (ICW1)
        x86_64::instructions::port::Port::<u8>::new(PIC1_COMMAND).write(0x11);
        x86_64::instructions::port::Port::<u8>::new(PIC2_COMMAND).write(0x11);

        // ICW2: Set vector offsets
        x86_64::instructions::port::Port::<u8>::new(PIC1_DATA).write(PIC1_OFFSET);
        x86_64::instructions::port::Port::<u8>::new(PIC2_DATA).write(PIC2_OFFSET);

        // ICW3: Tell Master PIC there is a slave at IRQ2
        x86_64::instructions::port::Port::<u8>::new(PIC1_DATA).write(0x04);
        // ICW3: Tell Slave PIC its cascade identity
        x86_64::instructions::port::Port::<u8>::new(PIC2_DATA).write(0x02);

        // ICW4: 8086 mode
        x86_64::instructions::port::Port::<u8>::new(PIC1_DATA).write(0x01);
        x86_64::instructions::port::Port::<u8>::new(PIC2_DATA).write(0x01);

        // Mask all interrupts except timer (IRQ0), keyboard (IRQ1), cascade (IRQ2), and mouse (IRQ12)
        x86_64::instructions::port::Port::<u8>::new(PIC1_DATA).write(0xF8); // 11111000 - enable IRQ0, IRQ1, IRQ2 (cascade)
        x86_64::instructions::port::Port::<u8>::new(PIC2_DATA).write(0xEF); // 11101111 - enable IRQ12 (mouse)
    }
}

/// Enable hardware interrupts
#[allow(dead_code)]
pub fn enable_interrupts() {
    x86_64::instructions::interrupts::enable();
}

/// Disable hardware interrupts
#[allow(dead_code)]
pub fn disable_interrupts() {
    x86_64::instructions::interrupts::disable();
}
