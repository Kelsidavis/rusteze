// src/process.rs

use x86_64::structures::idt::{InterruptDescriptorTable, InterruptStackFrame};
use crate::gdt;
use core::arch::asm;

/// Process Control Block (PCB) structure to manage process state.
#[derive(Debug)]
pub struct Pcb {
    /// Unique identifier for the process
    pub pid: usize,
    
    /// Current stack pointer when in kernel mode
    pub rsp0: u64,
    
    /// Stack pointers for different privilege levels
    pub rsp1: u64,
    pub rsp2: u64,
    
    /// Saved registers from user context (when switching to kernel)
    pub saved_regs: UserContextRegisters,
    
    /// Process state - running, ready, blocked, zombie
    pub state: ProcessState,
    
    /// Memory management information (page table pointer, etc.)
    // TODO: Add memory mapping info when paging is implemented
    
    /// File descriptor table for this process
    // TODO: Implement file descriptors later in Phase 6
}

/// User context registers that need to be saved/restored during context switching.
#[derive(Debug)]
pub struct UserContextRegisters {
    pub rax: u64,
    pub rbx: u64,
    pub rcx: u64,
    pub rdx: u64,
    pub rsi: u64,
    pub rdi: u64,
    pub rsp: u64,  // User stack pointer
    pub rip: u64,  // Instruction pointer (program counter)
    
    /// Flags register - contains status flags like carry bit, etc.
    pub rflags: u64,
}

/// Process states that a process can be in during its lifecycle.
#[derive(Debug, Clone, Copy)]
pub enum ProcessState {
    Running,
    Ready,
    Blocked,
    Zombie,
}

impl Default for UserContextRegisters {
    fn default() -> Self {
        Self {
            rax: 0,
            rbx: 0,
            rcx: 0,
            rdx: 0,
            rsi: 0,
            rdi: 0,
            rsp: 0,
            rip: 0,
            rflags: 0,
        }
    }
}

impl Default for ProcessState {
    fn default() -> Self {
        ProcessState::Ready
    }
}

/// Context switching functions to save and restore process state.
pub struct ContextSwitcher;

impl ContextSwitcher {
    /// Save the current user context into a Pcb structure.
    pub unsafe extern "C" fn save_context(pcb: &mut Pcb) {
        // We'll use inline assembly to capture registers
        asm!(
            "
                movq %0, rax
                movq %1, rbx 
                movq %2, rcx
                movq %3, rdx
                movq %4, rsi
                movq %5, rdi
                movq %6, rsp
                movq %7, rip
                movq %8, rflags
                
            ",
            in(reg) &mut pcb.saved_regs.rax,
            in(reg) &mut pcb.saved_regs.rbx,
            in(reg) &mut pcb.saved_regs.rcx,
            in(reg) &mut pcb.saved_regs.rdx,
            in(reg) &mut pcb.saved_regs.rsi,
            in(reg) &mut pcb.saved_regs.rdi,
            in(reg) &mut pcb.saved_regs.rsp,
            in(reg) &mut pcb.saved_regs.rip,
            in(reg) &mut pcb.saved_regs.rflags,
            
            // We're using a special register to store the Pcb pointer
            options(nostack, preserves_flags)
        );
    }

    /// Restore user context from a Pcb structure.
    pub unsafe extern "C" fn restore_context(pcb: &Pcb) {
        asm!(
            "
                movq rax, %0 
                movq rbx, %1
                movq rcx, %2
                movq rdx, %3
                movq rsi, %4
                movq rdi, %5
                movq rsp, %6
                movq rip, %7
                movq rflags, %8
                
            ",
            in(reg) &pcb.saved_regs.rax,
            in(reg) &pcb.saved_regs.rbx,
            in(reg) &pcb.saved_regs.rcx,
            in(reg) &pcb.saved_regs.rdx,
            in(reg) &pcb.saved_regs.rsi,
            in(reg) &pcb.saved_regs.rdi,
            in(reg) &pcb.saved_regs.rsp,
            in(reg) &pcb.saved_regs.rip,
            in(reg) &pcb.saved_regs.rflags,

            options(nostack, preserves_flags)
        );
    }

    /// Switch from one process to another.
    pub unsafe extern "C" fn switch_to(pcb: *mut Pcb) {
        // Save current context
        let mut current_pcb = (*pcb).clone();
        
        Self::save_context(&mut current_pcb);
        
        // Restore new context (this will return directly into the target process)
        if !(*pcb).state == ProcessState::Zombie {
            Self::restore_context(&*pcb);
            
            // If we get here, it means this was a switch back to an existing
            // running thread. The restore function doesn't return.
        }
    }

    /// Create a new process with the given initial state and stack pointer.
    pub fn create_process(pid: usize, entry_point: u64) -> Pcb {
        let mut pcb = Pcb {
            pid,
            rsp0: 0x1_0000 + (pid as u64 * 8 * 1024), // Stack for kernel mode
            rsp1: 0, 
            rsp2: 0,
            
            saved_regs: UserContextRegisters::default(),
            
            state: ProcessState::Ready,

            // Initialize memory management info - will be filled in later when paging is implemented
        };

        // Set up initial user context registers for the new process
        pcb.saved_regs.rip = entry_point;
        
        // Clear flags register (set to 0)
        pcb.saved_regs.rflags = 0;

        // We'll set rax, rbx etc. later when we need them

        pcb
    }
}

/// Initialize context switching functionality.
pub fn init_context_switching() {
    println!("Context switching system initialized");
}
