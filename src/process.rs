// src/process.rs

use x86_64::structures::idt::{InterruptDescriptorTable, InterruptStackFrame};
use crate::println;

/// Represents a process control block.
#[derive(Debug)]
pub struct ProcessControlBlock {
    /// Unique identifier for this process (PID)
    pub pid: u32,
    
    /// Current state of the process
    pub state: ProcessState,
    
    /// Stack pointer at time of context switch
    pub rsp: usize,
    
    /// Instruction pointer at time of context switch  
    pub rip: usize,
    
    /// General purpose registers saved during context switch
    pub rax: u64,
    pub rbx: u64,
    pub rcx: u64,
    pub rdx: u64,
    pub rsi: u64,
    pub rdi: u64,
    pub rbp: u64,
    
    /// Segment registers
    pub cs: u16,
    pub ds: u16,
    pub es: u16,
    pub fs: u16,
    pub gs: u16,
    pub ss: u16,
}

/// Possible states a process can be in.
#[derive(Debug, Clone, Copy)]
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
            rsp: 0,
            rip: 0,
            rax: 0, rbx: 0, rcx: 0, rdx: 0,
            rsi: 0, rdi: 0, rbp: 0,
            cs: 0, ds: 0, es: 0, fs: 0, gs: 0, ss: 0,
        }
    }
}

/// Context switching functions to save and restore process state.
pub struct ContextSwitcher;

impl ContextSwitcher {
    /// Save the current CPU context into a ProcessControlBlock
    pub fn save_context(pcb: &mut ProcessControlBlock) -> ! {
        unsafe extern "C" fn save_context_asm() -> ! {
            let mut rax: u64;
            let mut rbx: u64;
            let mut rcx: u64;
            let mut rdx: u64;
            let mut rsi: u64;
            let mut rdi: u64;
            let mut rsp: usize;
            
            // Use inline assembly to capture the current register values
            asm!(
                "mov {}, rax",
                "mov {}, rbx", 
                "mov {}, rcx",
                "mov {}, rdx",
                "mov {}, rsi",
                "mov {}, rdi",
                "mov {}, rsp",
                out(reg) rax,
                out(reg) rbx,
                out(reg) rcx,
                out(reg) rdx,
                out(reg) rsi,
                out(reg) rdi,
                out(reg) rsp,
                options(noreturn)
            );
            
            // Save the captured values to our PCB
            pcb.rax = rax;
            pcb.rbx = rbx;
            pcb.rcx = rcx;
            pcb.rdx = rdx;
            pcb.rsi = rsi;
            pcb.rdi = rdi;
            pcb.rsp = rsp as usize;
            
            // Save the instruction pointer (RIP)
            let rip: u64;
            asm!("mov {}, rip", out(reg) rip, options(noreturn));
            pcb.rip = rip as usize;
            
            // Get segment registers
            let cs: u16; 
            let ds: u16;
            let es: u16;
            let fs: u16;
            let gs: u16;
            let ss: u16;
            
            asm!(
                "mov {}, cs",
                "mov {}, ds",  
                "mov {}, es",
                "mov {}, fs",
                "mov {}, gs",
                "mov {}, ss",
                out(reg) cs,
                out(reg) ds,
                out(reg) es,
                out(reg) fs,
                out(reg) gs,
                out(reg) ss
            );
            
            pcb.cs = cs;
            pcb.ds = ds;
            pcb.es = es;
            pcb.fs = fs;
            pcb.gs = gs;
            pcb.ss = ss;
        }
        
        // Call the assembly function to save context
        unsafe {
            save_context_asm();
        }
    }

    /// Restore a previously saved CPU context from a ProcessControlBlock
    pub fn restore_context(pcb: &ProcessControlBlock) -> ! {
        unsafe extern "C" fn restore_context_asm() -> ! {
            // Use inline assembly to load the register values back into registers
            
            asm!(
                "mov rax, {}",
                "mov rbx, {}", 
                "mov rcx, {}",
                "mov rdx, {}",
                "mov rsi, {}",
                "mov rdi, {}",
                // Load stack pointer
                "mov rsp, {}",
                // Set instruction pointer (RIP)
                "jmp *{}",
                in(reg) pcb.rax,
                in(reg) pcb.rbx,
                in(reg) pcb.rcx,
                in(reg) pcb.rdx,
                in(reg) pcb.rsi,
                in(reg) pcb.rdi,
                in(reg) pcb.rsp as u64,
                // We can't directly jump to a register value, so we use an indirect jmp
                "mov rax, {}",
                "jmp *rax", 
                options(noreturn)
            );
        }
        
        unsafe {
            restore_context_asm();
        }
    }

    /// Switch from current process context to another's.
    pub fn switch_to(pcb: &ProcessControlBlock) -> ! {
        // Save the current state
        let mut current_pcb = ProcessControlBlock::default();
        Self::save_context(&mut current_pcb);
        
        // Restore the new state (this will never return)
        Self::restore_context(pcb);
    }
}

/// Initialize kernel threads and process management.
pub fn init_processes() {
    println!("Initializing process control block system...");
    
    let mut pcb = ProcessControlBlock::default();
    pcb.pid = 1;
    pcb.state = ProcessState::Running;
    
    // Mark as initialized
    unsafe { 
        crate::PROCESS_MANAGER.init(pcb);
    }
}

/// A simple scheduler that runs processes in round-robin fashion.
pub struct RoundRobinScheduler {
    current_process: Option<ProcessControlBlock>,
    ready_queue: Vec<ProcessControlBlock>,
}

impl RoundRobinScheduler {
    pub fn new() -> Self {
        Self {
            current_process: None,
            ready_queue: vec![],
        }
    }

    /// Add a process to the scheduler's queue
    pub fn add(&mut self, pcb: ProcessControlBlock) {
        // Set state to Ready and enqueue it
        let mut proc = pcb;
        proc.state = ProcessState::Ready;
        
        if !self.ready_queue.contains(&proc) {
            self.ready_queue.push(proc);
        }
    }

    /// Get the next process in round-robin order
    pub fn get_next_process(&mut self) -> Option<ProcessControlBlock> {
        // If no processes, return None
        if self.ready_queue.is_empty() {
            return None;
        }
        
        let mut current = 0;
        
        // Find and remove the next process from queue (round-robin)
        for i in 0..self.ready_queue.len() {
            if &self.ready_queue[i] == self.current_process.as_ref().unwrap_or(&ProcessControlBlock::default()) {
                current = i + 1; 
                break;
            }
        }

        // Wrap around to beginning
        let next_idx = (current) % self.ready_queue.len();
        
        Some(self.ready_queue.remove(next_idx))
    }

    /// Set the currently running process
    pub fn set_current(&mut self, pcb: ProcessControlBlock) {
        self.current_process = Some(pcb);
    }
}

/// A global manager for processes.
pub struct ProcessManager {
    current_pcb: Option<ProcessControlBlock>,
    scheduler: RoundRobinScheduler,
}

impl ProcessManager {
    /// Create a new process manager
    pub fn new() -> Self {
        Self {
            current_pcb: None,
            scheduler: RoundRobinScheduler::new(),
        }
    }

    /// Initialize the process manager with an initial PCB
    pub fn init(&mut self, pcb: ProcessControlBlock) {
        // Set up first running process
        self.current_pcb = Some(pcb);
        
        println!("Process management system initialized");
    }

    /// Add a new process to be scheduled
    pub fn add_process(&mut self, pcb: ProcessControlBlock) {
        self.scheduler.add(pcb);
    }
    
    /// Get the next ready-to-run process (round-robin)
    pub fn get_next_process(&mut self) -> Option<ProcessControlBlock> {
        // Try to find a new running process
        let mut proc = match self.scheduler.get_next_process() {
            Some(proc) => proc,
            None => return None,  // No processes ready
        };
        
        // Update current state and scheduler info
        self.current_pcb = Some(proc.clone());
        self.scheduler.set_current(proc);
        
        println!("Switching to process PID {}", proc.pid);

        Some(proc)
    }

    /// Get the currently running process (if any)
    pub fn get_current(&self) -> Option<&ProcessControlBlock> {
        self.current_pcb.as_ref()
    }
}

// Global instance of ProcessManager
pub static mut PROCESS_MANAGER: ProcessManager = ProcessManager::new();

/// A simple test function to demonstrate context switching.
#[allow(dead_code)]
fn test_context_switch() {
    println!("Testing process control block and context switch...");
    
    // Create two processes with different PIDs
    let mut proc1 = ProcessControlBlock::default();
    proc1.pid = 1;
    proc1.state = ProcessState::Ready;
    proc1.rax = 0x123456789ABCDEF0u64; 
    proc1.rip = 0xCAFEBABE;

    let mut proc2 = ProcessControlBlock::default();
    proc2.pid = 2;
    proc2.state = ProcessState::Ready;
    proc2.rax = 0xFEDCBA9876543210u64; 
    proc2.rip = 0xDEADBEEF;

    // Add them to the scheduler
    unsafe {
        PROCESS_MANAGER.add_process(proc1);
        PROCESS_MANAGER.add_process(proc2);

        let next_proc = PROCESS_MANAGER.get_next_process();
        
        if let Some(pcb) = next_proc {
            println!("Switching context: PID {}", pcb.pid);
            
            // Simulate switching back to the original process
            unsafe { 
                ContextSwitcher::switch_to(&pcb);  // This should never return!
            }
        } else {
            println!("No processes available for scheduling");
        }

    }
}
