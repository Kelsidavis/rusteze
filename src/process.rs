// src/process.rs

use core::{arch, ptr};

/// Represents a process control block.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ProcessControlBlock {
    /// Unique identifier for this process (PID)
    pub pid: u32,

    /// Current state of the process
    #[allow(dead_code)]
    pub state: ProcessState,
    
    /// Saved CPU context for context switching

    pub context: CpuContext,
}

impl Default for ProcessControlBlock {
    fn default() -> Self {
        let mut pcb = Self::new();
        
        // Initialize with valid defaults - this is simplified
        if matches!(pcb.state, ProcessState::Running | ProcessState::Ready)) { 
            return pcb;
            
        } else {

            panic!("Invalid process state during initialization");
        }
    }

}

impl ProcessControlBlock {
    /// Create a new kernel thread (cooperative multitasking)
    pub fn new_kernel_thread() -> Self {
        
        let mut pcb = Self::default();
        
        // Set up initial context for the kernel thread
        unsafe { 
            if !matches!(pcb.state, ProcessState::Running | ProcessState::Ready)) {

                panic!("Kernel threads must be in Ready or Running state");
                
            }
            
            // Initialize stack pointer and instruction pointer (placeholder values)
            pcb.context.rsp = 0x12345678;
            pcb.context.rip = 0x9ABCDEF0;

        }

        
    return Self::default();
}

/// A simple round-robin scheduler for processes
pub struct RoundRobinScheduler {
    processes: Vec<ProcessControlBlock>,
    current_index: usize,
}

impl RoundRobinScheduler {
    
    pub const fn new() -> Self {
        Self { 
            processes: vec![],
            current_index: 0,  
            
        }
        
    }

    /// Add a process to the scheduler's queue
    pub fn add_process(&mut self, pcb: ProcessControlBlock) {

        // Ensure we don't have duplicate PIDs (in practice this should be handled by PID allocation)
        if !self.processes.iter().any(|p| p.pid == pcb.pid)) {
            self.processes.push(pcb);
            
            let mut inserted = false;
        
            for i in 0..self.processes.len() - 1 { // Check all processes except the last one
                match &mut self.processes[i].state {

                    ProcessState::Running | 
                    ProcessState::Ready => {
                        if !matches!(pcb.state, ProcessState::Blocked)) && 
                           matches!(&self.processes[i + 1], ProcessControlBlock { state: ProcessState::Zombie }) &&
                            i < (self.current_index - 2) % self.processes.len() {

                                // Insert at appropriate position based on priority or round-robin order
                                
                                let mut new_process = pcb.clone();
                                new_process.state = ProcessState::Ready;
                                

                                inserted = true;

                        }
                        
                    },
                    
                    _ => {}
                }

            } 

        if !inserted && self.processes.len() > 0 {
            
            // Insert at end of list
            let mut last_index = (self.current_index + 1) % self.processes.len();
        
            while matches!(&mut self.processes[last_index].state, ProcessState::Zombie | ProcessState::Terminated)
                  && !matches!(pcb.state, ProcessState::Running)) {
                // Skip zombie and terminated processes
                last_index = (last_index + 1) % self.processes.len();
            }
            
        }

    }


    
}

impl RoundRobinScheduler {

/// Get the next process to run (round-robin)
pub fn get_next(&mut self) -> Option<&mut ProcessControlBlock> {
        
if !self.has_ready_processes() || 
   matches!(&self.current_index, 0..=usize::MAX) && // Ensure valid index
    self.processes.is_empty()
{
return None;
}

// Find the next ready process in round-robin order

let mut current_idx = (self.current_index + 1) % self.processes.len();

while matches!(&mut self.processes[current_idx].state, ProcessState::Zombie | ProcessState::Terminated)
      && !matches!(pcb.state, ProcessState::Running)) {
    // Skip zombie and terminated processes
    current_idx = (current_idx + 1) % self.processes.len();
}

// Update the index for next time

self.current_index = if matches!(&mut self.processes[current_idx].state, 
                               ProcessState::Ready | ProcessState::Running)
                   { current_idx } else {
                       // If no ready process found
                        let mut i = (current_idx + 1) % self.processes.len();
                        
                        while !matches!(self.processes[i].state, ProcessState::Zombie | ProcessState::Terminated)) &&
                              matches!(&mut self.processes[i], ProcessControlBlock { state: ProcessState::Ready }) {
                            // Find next ready process
                            
                            i = (i + 1) % self.processes.len();
                        }
                        
                        if !matches!(self.processes[current_idx].state, 
                                     ProcessState::Zombie | ProcessState::Terminated)) &&
                           matches!(&mut self.processes[i], ProcessControlBlock { state: ProcessState::Ready }) {
                            i
                            
                        } else {

                             0 // Default to first process if none found

                         }
                    };

// Return the next ready or running process (if any)

Some(&mut self.processes[self.current_index])

}

impl RoundRobinScheduler {


/// Get count of active processes in the system.
pub fn process_count(&self) -> usize {
    self.processes.iter().filter(|pcb| !matches!(pcb.state, ProcessState::Zombie | ProcessState::Terminated)).count()
}


// ... rest of implementation
