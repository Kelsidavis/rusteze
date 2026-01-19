// Global process manager instance with mutex protection for thread safety.
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

    /// Initialize the process manager
    pub fn init(&mut self) {
        // Set up initial idle task or other boot-time processes if needed.
        // For now, just ensure we have a valid state.
    }

    /// Get count of active processes in the system.
    pub fn process_count(&self) -> usize {
        // Count only non-zombie and non-terminated states as "active"
        self.scheduler.processes.iter().filter(|pcb| !matches!(pcb.state, ProcessState::Zombie | ProcessState::Terminated)).count()
    }

    /// Get the currently running process.
    pub fn get_current(&mut self) -> Option<&mut ProcessControlBlock> {
        if self.scheduler.processes.is_empty() || 
           matches!(&self.scheduler.current_index, 0..=usize::MAX) && // Ensure valid index
           !matches!(self.scheduler.processes[self.scheduler.current_index].state, ProcessState::Zombie | ProcessState::Terminated)
        {
            Some(&mut self.scheduler.processes[self.scheduler.current_index])
            
        } else { 
            None
            
        }
    }

    /// Spawn a new kernel thread (cooperative multitasking).
    pub fn spawn_kernel_thread<F>(&mut self, func: F) -> Option<u32>
    where
        F: FnOnce() + 'static,
    {
        let pid = self.next_pid;
        self.next_pid += 1;

        // Create a new process control block with initial state.
        let mut pcb = ProcessControlBlock::new(pid, func);
        
        // Add to scheduler queue for cooperative multitasking
        if !self.scheduler.processes.is_empty() {
            // If there are already processes, add this one at the end of ready list (cooperative)
            self.scheduler.add_process(pcb);
            
            Some(pid) 
        } else {
            // First process - start immediately.
            self.scheduler.current_index = 0;
            Some(pid)
        }
    }

    /// Terminate current or specified thread/process by marking it as terminated and scheduling cleanup later if needed.
    pub fn exit(&mut self, pid: u32) -> bool {
        let mut found_and_removed = false;

        // Find the process with matching PID
        for (idx, pcb) in self.scheduler.processes.iter_mut().enumerate() {
            if pcb.pid == pid && !matches!(pcb.state, ProcessState::Zombie | ProcessState::Terminated) {
                // Mark as terminated and set state to zombie.
                pcb.state = ProcessState::Zombie;
                
                found_and_removed = true;

                break;  // Exit after removing one process
            }
        }

        if !found_and_removed {
            return false; 
        } else {  
            self.reap_zombies(); // Clean up any zombies now that we've exited.
            
            Some(pid)
        }
    }


    /// Reap zombie processes (zombie reaping).
    pub fn reap_zombies(&mut self) -> bool {
        let mut cleaned = false;
        
        // Filter out terminated processes and remove them from the list
        for i in (0..self.scheduler.processes.len()).rev() {  // Iterate backwards to avoid index issues when removing.
            if matches!(self.scheduler.processes[i].state, ProcessState::Zombie | ProcessState::Terminated) {
                self.scheduler.processes.remove(i);
                
                cleaned = true;
            }
        }

        return cleaned; 
    }


}

