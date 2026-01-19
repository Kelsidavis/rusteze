// src/process.rs
// Process management and context switching implementation

use core::arch::asm;
use alloc::vec::Vec;

/// Represents a process control block.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ProcessControlBlock {
    /// Unique identifier for this process (PID)
    pub pid: u32,

    /// Current state of the process
    pub state: ProcessState,

    /// Saved CPU context for context switching
    pub context: CpuContext,
}

/// CPU context saved during context switch
#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(C)]
pub struct CpuContext {
    // General purpose registers
    pub r15: u64,
    pub r14: u64,
    pub r13: u64,
    pub r12: u64,
    pub r11: u64,
    pub r10: u64,
    pub r9: u64,
    pub r8: u64,
    pub rdi: u64,
    pub rsi: u64,
    pub rbp: u64,
    pub rbx: u64,
    pub rdx: u64,
    pub rcx: u64,
    pub rax: u64,

    // Stack pointer
    pub rsp: u64,

    // Instruction pointer (return address)
    pub rip: u64,

    // RFLAGS register
    pub rflags: u64,

    // Segment selectors
    pub cs: u64,
    pub ss: u64,
}

impl Default for CpuContext {
    fn default() -> Self {
        Self {
            r15: 0, r14: 0, r13: 0, r12: 0,
            r11: 0, r10: 0, r9: 0, r8: 0,
            rdi: 0, rsi: 0, rbp: 0, rbx: 0,
            rdx: 0, rcx: 0, rax: 0,
            rsp: 0, rip: 0, rflags: 0,
            cs: 0, ss: 0,
        }
    }
}

/// Possible states a process can be in.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ProcessState {
    /// Ready to run (waiting for CPU time)
    Ready,

    /// Currently running on the CPU
    Running,

    /// Waiting for an event or resource
    Blocked,

    /// Terminated and waiting for cleanup
    Zombie,
}

impl Default for ProcessControlBlock {
    fn default() -> Self {
        Self {
            pid: 0,
            state: ProcessState::Ready,
            context: CpuContext::default(),
        }
    }
}

/// Context switching implementation using proper naked functions
pub struct ContextSwitcher;

impl ContextSwitcher {
    /// Save the current CPU context to the given location
    ///
    /// # Safety
    /// This function saves all callee-saved registers and the return address
    /// to the provided context pointer. The caller must ensure the pointer is valid.
    #[inline(never)]
    pub unsafe fn save_context(ctx: *mut CpuContext) -> u64 {
        let ret: u64;

        asm!(
            // Save all general purpose registers
            "mov [rdi + 0], r15",
            "mov [rdi + 8], r14",
            "mov [rdi + 16], r13",
            "mov [rdi + 24], r12",
            "mov [rdi + 32], r11",
            "mov [rdi + 40], r10",
            "mov [rdi + 48], r9",
            "mov [rdi + 56], r8",

            // Save rdi by using a temp register
            "mov r8, rdi",
            "mov [rdi + 64], r8",   // Save original rdi

            "mov [rdi + 72], rsi",
            "mov [rdi + 80], rbp",
            "mov [rdi + 88], rbx",
            "mov [rdi + 96], rdx",
            "mov [rdi + 104], rcx",
            "mov [rdi + 112], rax",

            // Save stack pointer (before the call)
            "lea r8, [rsp + 8]",      // Skip return address
            "mov [rdi + 120], r8",    // rsp

            // Save return address as rip
            "mov r8, [rsp]",
            "mov [rdi + 128], r8",    // rip

            // Save rflags
            "pushfq",
            "pop r8",
            "mov [rdi + 136], r8",    // rflags

            // Save segment registers
            "mov r8, cs",
            "mov [rdi + 144], r8",    // cs
            "mov r8, ss",
            "mov [rdi + 152], r8",    // ss

            // Return 0 to indicate we're saving (not resuming)
            "xor rax, rax",

            inout("rdi") ctx => _,
            out("rax") ret,
            out("r8") _,
            options(nostack)
        );

        ret
    }

    /// Restore CPU context from the given location
    ///
    /// # Safety
    /// This function never returns - it jumps to the saved instruction pointer.
    /// The caller must ensure the context pointer is valid and points to a
    /// previously saved context.
    #[inline(never)]
    pub unsafe fn restore_context(ctx: *const CpuContext) -> ! {
        asm!(
            // Restore general purpose registers (except rsp and rip which we handle specially)
            "mov r15, [rdi + 0]",
            "mov r14, [rdi + 8]",
            "mov r13, [rdi + 16]",
            "mov r12, [rdi + 24]",
            "mov r11, [rdi + 32]",
            "mov r10, [rdi + 40]",
            "mov r9, [rdi + 48]",
            "mov r8, [rdi + 56]",

            "mov rsi, [rdi + 72]",
            "mov rbp, [rdi + 80]",
            "mov rbx, [rdi + 88]",
            "mov rdx, [rdi + 96]",
            "mov rcx, [rdi + 104]",
            "mov rax, [rdi + 112]",

            // Restore rflags
            "mov r8, [rdi + 136]",
            "push r8",
            "popfq",

            // Set up return by pushing rip onto the stack
            "mov r8, [rdi + 128]",    // Load saved rip
            "mov rsp, [rdi + 120]",   // Restore stack pointer
            "push r8",                 // Push return address

            // Restore rdi last since we've been using it
            "mov rdi, [rdi + 64]",

            // Return to saved rip (pops the address we just pushed)
            "ret",

            in("rdi") ctx,
            options(noreturn)
        );
    }

    /// Switch from current context to another context
    ///
    /// # Safety
    /// This saves the current context and then restores the new context.
    /// The function returns 0 when first called (after save), and returns 1
    /// when resumed (after restore).
    #[inline(never)]
    pub unsafe fn switch_context(old_ctx: *mut CpuContext, new_ctx: *const CpuContext) -> u64 {
        let ret = Self::save_context(old_ctx);

        if ret == 0 {
            // First time through - we just saved, now restore the new context
            Self::restore_context(new_ctx);
        } else {
            // We've been resumed - return 1
            1
        }
    }
}

/// A simple round-robin scheduler for processes
pub struct RoundRobinScheduler {
    processes: Vec<ProcessControlBlock>,
    current_index: usize,
}

impl RoundRobinScheduler {
    pub const fn new() -> Self {
        Self {
            processes: Vec::new(),
            current_index: 0,
        }
    }

    /// Add a process to the scheduler's queue
    pub fn add_process(&mut self, pcb: ProcessControlBlock) {
        self.processes.push(pcb);
    }

    /// Get the next process to run (round-robin)
    pub fn get_next(&mut self) -> Option<&mut ProcessControlBlock> {
        if self.processes.is_empty() {
            return None;
        }

        // Move to next process in round-robin fashion
        self.current_index = (self.current_index + 1) % self.processes.len();
        Some(&mut self.processes[self.current_index])
    }

    /// Get the currently running process
    pub fn get_current(&mut self) -> Option<&mut ProcessControlBlock> {
        if self.processes.is_empty() {
            return None;
        }

        Some(&mut self.processes[self.current_index])
    }

    /// Get current process count
    pub fn process_count(&self) -> usize {
        self.processes.len()
    }
}

/// Global process manager
pub struct ProcessManager {
    scheduler: RoundRobinScheduler,
    next_pid: u32,
}

impl ProcessManager {
    pub const fn new() -> Self {
        Self {
            scheduler: RoundRobinScheduler::new(),
            next_pid: 1,
        }
    }

    /// Initialize the process manager
    pub fn init(&mut self) {
        // Create an initial idle process
        let mut idle = ProcessControlBlock::default();
        idle.pid = 0;
        idle.state = ProcessState::Running;

        self.scheduler.add_process(idle);
    }

    /// Spawn a new kernel thread
    ///
    /// # Safety
    /// The entry point must be a valid function pointer that never returns.
    /// The stack must be properly aligned and valid.
    pub unsafe fn spawn_kernel_thread(
        &mut self,
        entry_point: extern "C" fn() -> !,
        stack_top: u64,
    ) -> u32 {
        let pid = self.next_pid;
        self.next_pid += 1;

        let mut pcb = ProcessControlBlock::default();
        pcb.pid = pid;
        pcb.state = ProcessState::Ready;

        // Set up initial context
        pcb.context.rsp = stack_top;
        pcb.context.rip = entry_point as u64;
        pcb.context.rflags = 0x202; // IF (interrupts enabled) + reserved bit 1

        // Set up segment registers (kernel mode)
        pcb.context.cs = 0x08;  // Kernel code segment
        pcb.context.ss = 0x10;  // Kernel data segment

        self.scheduler.add_process(pcb);

        pid
    }

    /// Perform a context switch to the next ready process
    ///
    /// # Safety
    /// This should only be called from an interrupt handler or with interrupts disabled
    pub unsafe fn schedule(&mut self) {
        // Get current and next process
        let current = match self.scheduler.get_current() {
            Some(p) => p as *mut ProcessControlBlock,
            None => return,
        };

        let next = match self.scheduler.get_next() {
            Some(p) => p as *mut ProcessControlBlock,
            None => return,
        };

        // Don't switch if it's the same process
        if current == next {
            return;
        }

        // Update states
        (*current).state = ProcessState::Ready;
        (*next).state = ProcessState::Running;

        // Perform context switch
        let old_ctx = &mut (*current).context as *mut CpuContext;
        let new_ctx = &(*next).context as *const CpuContext;

        ContextSwitcher::switch_context(old_ctx, new_ctx);
    }

    /// Get the number of processes
    pub fn process_count(&self) -> usize {
        self.scheduler.process_count()
    }
}

// Note: Global process manager would be initialized in main
// For now, we just provide the types for when it's needed
