use crate::process::{ProcessControlBlock, ProcessState};
use crate::scheduler::RoundRobinScheduler;
use core::sync::atomic::{AtomicBool, Ordering};

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
    ///
    /// This sets up internal state and prepares for spawning threads.
    pub fn init(&mut self) {}

    /// Get count of active processes in the system.
    pub fn process_count(&self) -> usize {
        self.scheduler.process_count()
    }

    /// Add a new kernel thread (process control block).
    ///
    /// This is used to spawn cooperative multitasking threads within the kernel itself.
    pub fn spawn_kernel_thread(&mut self, pcb: ProcessControlBlock) {
        // Assign PID
        let pid = self.next_pid;
        self.next_pid += 1;

        // Set process state and add it to scheduler queue
        let mut new_pcb = pcb;
        new_pcb.state = ProcessState::Ready;
        new_pcb.pid = pid;

        self.scheduler.add_process(new_pcb);
    }

    /// Terminate a running thread by marking its state as `Zombie`.
    ///
    /// This function is called when a process exits or fails.
    pub fn exit(&mut self, pcb: &ProcessControlBlock) {
        // Find the PCB in scheduler and mark it as zombie
        if let Some(index) = self.scheduler.processes.iter().position(|p| p.pid == pcb.pid) {
            self.scheduler.processes[index].state = ProcessState::Zombie;
        }
    }

    /// Reap all terminated processes (zombies).
    ///
    /// This cleans up any exited threads and frees their resources.
    pub fn reap_zombies(&mut self) -> bool {
        let mut reaped = false;

        // Filter out zombie processes
        self.scheduler.processes.retain(|pcb| match pcb.state {
            ProcessState::Zombie => { 
                reaped = true;
                false  // Remove from list after cleanup
            },
            _ => true, // Keep all non-zombies
        });

        reaped
    }

    /// Get the currently running process.
    pub fn get_current(&mut self) -> Option<&mut ProcessControlBlock> {
        if self.scheduler.processes.is_empty() {
            return None;
        }
        
        Some(&mut self.scheduler.processes[self.scheduler.current_index])
    }

    /// Retrieve next ready-to-run task from scheduler (round-robin).
    pub fn get_next_process(&mut self) -> Option<&mut ProcessControlBlock> {
        if self.scheduler.processes.is_empty() {
            return None;
        }
        
        // Move to the next process in round-robin fashion
        let current_index = &self.scheduler.current_index;

        // Advance index, wrap around at end of list.
        self.scheduler.current_index =
            (current_index + 1) % self.scheduler.processes.len();

        Some(&mut self.scheduler.processes[self.scheduler.current_index])
    }
}

/// Global instance for use in interrupt handlers and other contexts
pub static mut PROCESS_MANAGER: Option<ProcessManager> = None;

// Initialize the global process manager at boot time.
#[inline(never)]
pub fn init_process_manager() {
    unsafe {
        PROCESS_MANAGER = Some(ProcessManager::new());
    }

    // Ensure we can spawn threads immediately after initialization
}
