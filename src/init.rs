//! Init process (PID 1) - the first userspace process
//!
//! The init process is responsible for:
//! - Being the first process spawned by the kernel
//! - Launching the shell or other userspace programs
//! - Reaping zombie children (waiting for terminated processes)
//!
//! Note: This module is infrastructure for future userspace program loading.
//! It will be used once the Ring 0 -> Ring 3 transition is implemented.

#![allow(dead_code)]

use crate::process::PROCESS_MANAGER;

/// Entry point for the init process
///
/// This function will be called as the first userspace process (PID 1).
/// For now, it's a placeholder that demonstrates the concept.
#[allow(dead_code)]
pub fn init_main() -> ! {
    // For now, just loop forever
    // In a real implementation, this would:
    // 1. Set up the environment
    // 2. Launch the shell
    // 3. Wait for child processes and reap zombies
    //
    // Note: We can't call syscalls directly from kernel code.
    // This function would need to run in userspace with proper
    // privilege level transitions.
    loop {
        // Yield to other processes
        unsafe {
            core::arch::asm!("hlt");
        }
    }
}

/// Spawn the init process
///
/// This function should be called by the kernel during boot to create
/// the first userspace process (PID 1).
pub fn spawn_init_process() -> Result<(), &'static str> {
    // Get the process manager
    let pm = PROCESS_MANAGER.lock();

    // For now, we don't actually spawn a real process because we need:
    // 1. The ELF loader to load the init binary
    // 2. Page tables set up for userspace
    // 3. User stack allocated and mapped
    // 4. Entry point set to init_main()

    // This is a placeholder that shows the infrastructure is ready
    drop(pm);

    Ok(())
}

/// Simple init process implementation as embedded shellcode
///
/// This is a minimal init process that can be loaded without ELF parsing.
/// It demonstrates that the init process infrastructure is in place.
#[allow(dead_code)]
pub static INIT_SHELLCODE: &[u8] = &[
    // This would contain x86-64 machine code for a minimal init process
    // For now, it's just a placeholder
    // Real implementation would include:
    // - syscall to write "Init started"
    // - loop forever with hlt instruction
];
