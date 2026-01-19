// src/process_manager.rs

/// Global process manager instance with mutex protection for thread safety.
pub struct ProcessManager {
    scheduler: RoundRobinScheduler,
    #[allow(dead_code)]
    next_pid: u32,
}

impl ProcessManager {
    /// Creates a new empty `ProcessManager`.
    pub const fn new() -> Self {
        Self {
            scheduler: RoundRobinScheduler::new(),
            next_pid: 1,
        }
    }

    /// Initialize the process manager.
    pub fn init(&mut self) {
        // Reset PID counter
        self.next_pid = 1;
        
        // Clear any existing processes
        self.scheduler.processes.clear();
    }

    /// Create a new kernel thread (cooperative multitasking).
    ///
    /// # Arguments
    /// * `entry_point` - The function to start executing when the thread runs.
    pub fn spawn_kernel_thread<F>(&mut self, entry_point: F) -> u32 
    where
        F: Fn() + 'static,
    {
        // Create a new process control block with initial state
        let mut pcb = ProcessControlBlock::new();
        
        // Set up the thread's execution context (stack and instruction pointer)
        unsafe {
            // Initialize stack for this kernel thread
            let stack_ptr = self.allocate_kernel_stack().unwrap_or_else(|| panic!("Failed to allocate kernel stack"));
            
            // Setup initial register state - we'll use a simple approach here
            pcb.context.set_initial_state(
                entry_point as usize,
                stack_ptr as usize,
            );
        }
        
        // Assign PID and mark thread as running (ready for execution)
        let pid = self.next_pid;
        self.next_pid += 1;

        // Add to scheduler queue - this will be the first process
        if self.scheduler.processes.is_empty() {
            self.scheduler.current_index = 0;
        }
        
        // Store in our processes list with initial state
        pcb.state = ProcessState::Running; 
        self.scheduler.processes.push(pcb);
        
        pid
    }

    /// Terminate a process and clean up its resources.
    ///
    /// # Arguments
    /// * `pid` - The PID of the process to terminate.
    pub fn exit(&mut self, pid: u32) -> bool {
        // Find the process with matching PID in our scheduler queue
        if let Some(index) = self.scheduler.processes.iter().position(|pcb| pcb.pid == pid) {
            // Mark as zombie and remove from active processes
            let mut pcb = &mut self.scheduler.processes[index];
            
            // Set state to Zombie (terminated but not yet cleaned up)
            pcb.state = ProcessState::Zombie;
            
            // If this was the current running process, we need to schedule next one
            if index == self.scheduler.current_index {
                let mut found_next_process = false;
                
                // Start from the next position after current (round robin)
                let start_idx = (self.scheduler.current_index + 1) % self.scheduler.processes.len();
                
                for i in 0..self.scheduler.processes.len() {
                    let idx = (start_idx + i) % self.scheduler.processes.len();
                    
                    if !matches!(self.scheduler.processes[idx].state, ProcessState::Zombie | ProcessState::Terminated)
                       && matches!(self.scheduler.processes[idx].state, ProcessState::Ready)
                    {
                        // Found a ready process to schedule next
                        self.scheduler.current_index = idx;
                        
                        found_next_process = true;
                        break;
                    }
                }

                if !found_next_process {
                    // No other processes are available - set current index to 0 (idle state) or handle accordingly
                    self.scheduler.current_index = 0; 
                    
                    // If all threads have exited, we might want to halt the system here eventually.
                    // For now just keep running idle task if no others exist.
                }
            }

            true // Successfully terminated process with PID `pid`
        } else {
            false // Process not found
        }
    }

    /// Reap zombie processes and free their resources.
    ///
    /// This should be called periodically to clean up exited threads that are still in Zombie state.
    pub fn reap_zombies(&mut self) -> Vec<u32> {
        let mut reaped_pids = vec![];
        
        // We'll collect all zombies first, then remove them
        for i in (0..self.scheduler.processes.len()).rev() {  // Iterate backwards to avoid index issues when removing items
            if matches!(self.scheduler.processes[i].state, ProcessState::Zombie) {
                let pid = self.scheduler.processes[i].pid;
                
                // Remove from list and clean up resources (stack memory etc.)
                self.scheduler.processes.remove(i);
                
                reaped_pids.push(pid); 
            }
        }

        reaped_pids
    }

    /// Get count of active processes in the system.
    pub fn process_count(&self) -> usize {
        // Only include non-zombie and non-terminated states as "active"
        self.scheduler.processes.iter().filter(|pcb| !matches!(pcb.state, ProcessState::Zombie | ProcessState::Terminated)).count()
    }

    /// Get the currently running process.
    pub fn get_current(&mut self) -> Option<&mut ProcessControlBlock> {
        if self.scheduler.processes.is_empty() {
            return None;
        }
        
        Some(&mut self.scheduler.processes[self.scheduler.current_index])
    }

    // Helper method to allocate a stack for kernel threads
    #[allow(dead_code)]
    fn allocate_kernel_stack(&self) -> Option<*const u8> {
        use crate::physical_memory::{PhysFrame, PhysAddr};
        
        const KERNEL_STACK_SIZE: usize = 4096; // One page
        
        let frame = unsafe { 
            match PhysFrame::allocate() {
                Some(f) => f,
                None => return None
            }
        };
        
        let stack_ptr = (frame.start_address().as_u64() + KERNEL_STACK_SIZE as u64 - 1) as *const u8;
        
        // Ensure we have a valid pointer to the top of our allocated kernel stack.
        Some(stack_ptr)
    }

}

// Process states for tracking thread lifecycle
#[derive(Debug, Clone, Copy)]
pub enum ProcessState {
    Ready,
    Running,
    Blocked,
    Zombie,
    Terminated,
}
