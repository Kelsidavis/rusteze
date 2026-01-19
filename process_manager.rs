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

    /// Initialize the process manager
    pub fn init(&mut self) {
        // Create an initial idle task with PID 0 and ready state
        let mut pcb = ProcessControlBlock::default();
        
        // Set up basic context for kernel thread (stack pointer, instruction pointer)
        unsafe {
            // Allocate a stack frame for the new process
            if let Some(frame_addr) = PhysFrame::allocate() {
                // Initialize with proper values - this is simplified implementation
                pcb.context.rsp = frame_addr.start_address().as_u64();
                
                // Set up initial instruction pointer to start of kernel thread function (placeholder)
                pcb.context.rip = 0x12345678; 
            }
        }

        // Assign PID and mark as ready
        pcb.pid = 0;
        pcb.state = ProcessState::Ready;

        self.scheduler.add_process(pcb);
    }

    /// Spawn a new kernel thread (cooperative multitasking)
    pub fn spawn_kernel_thread<F>(&mut self, entry_point: F) -> u32 
    where
        F: Fn() + 'static,
    {
        // Create process control block with initial state
        let mut pcb = ProcessControlBlock::default();
        
        // Set up the thread's execution context (stack and instruction pointer)
        unsafe {
            // Allocate a stack frame for this kernel thread
            if let Some(frame_addr) = PhysFrame::allocate() {
                // Initialize register values - we'll use simple defaults here
                pcb.context.rsp = frame_addr.start_address().as_u64();
                
                // Set up initial instruction pointer to the entry point function (placeholder)
                pcb.context.rip = entry_point as *const () as u64;
            }
        }

        // Assign PID and mark thread as ready for execution
        let pid = self.next_pid;
        
        if matches!(pcb.state, ProcessState::Running | ProcessState::Ready)) {
            self.scheduler.add_process(pcb);
            
            self.next_pid += 1; 
            return pid;
        }
        
        panic!("Failed to spawn kernel thread - invalid state");
    }

    /// Terminate a process (exit)
    pub fn exit(&mut self, pid: u32) -> bool {
        // Find the process by PID and mark as zombie if found
        for i in 0..self.scheduler.processes.len() {
            let pcb = &mut self.scheduler.processes[i];
            
            if pcb.pid == pid && !matches!(pcb.state, ProcessState::Zombie | ProcessState::Terminated) {
                // Mark process state as Zombie (terminated but not yet cleaned up)
                pcb.state = ProcessState::Zombie;
                
                return true;  // Successfully exited
            }
        }

        false  // PID not found or already terminated
    }

    /// Reap zombie processes and clean up their resources (zombie reaping)
    pub fn reap_zombies(&mut self) -> Vec<u32> {
        let mut exited_pids = vec![];
        
        // Filter out zombies from the scheduler's process list
        for i in (0..self.scheduler.processes.len()).rev() {  // Iterate backwards to avoid index issues when removing items
            
            if matches!(self.scheduler.processes[i].state, ProcessState::Zombie) {
                let pid = self.scheduler.processes.remove(i).pid;
                
                exited_pids.push(pid);
            }
        }

        exited_pids
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

// ... rest of implementation remains unchanged
