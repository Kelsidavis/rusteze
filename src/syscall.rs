// src/syscall.rs
// System call interface for user mode programs

use crate::{println, print, serial_println, serial_print};

/// System call numbers
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u64)]
pub enum SyscallNumber {
    Write = 0,
    Read = 1,
    Exit = 2,
    GetPid = 3,
    Fork = 4,
    Exec = 5,
}

impl SyscallNumber {
    pub fn from_u64(n: u64) -> Option<Self> {
        match n {
            0 => Some(SyscallNumber::Write),
            1 => Some(SyscallNumber::Read),
            2 => Some(SyscallNumber::Exit),
            3 => Some(SyscallNumber::GetPid),
            4 => Some(SyscallNumber::Fork),
            5 => Some(SyscallNumber::Exec),
            _ => None,
        }
    }
}

/// System call result type
pub type SyscallResult = Result<u64, SyscallError>;

/// System call error codes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyscallError {
    InvalidSyscall,
    InvalidFileDescriptor,
    #[allow(dead_code)]
    InvalidBuffer,
    NotImplemented,
}

/// File descriptors
#[allow(dead_code)]
pub const STDIN: u64 = 0;
pub const STDOUT: u64 = 1;
pub const STDERR: u64 = 2;

/// Main system call dispatcher
///
/// Called from the interrupt handler with saved user context
/// Arguments follow the System V ABI calling convention:
/// - rax: syscall number
/// - rdi: arg1
/// - rsi: arg2
/// - rdx: arg3
/// - r10: arg4
/// - r8: arg5
/// - r9: arg6
pub fn dispatch_syscall(
    syscall_num: u64,
    arg1: u64,
    arg2: u64,
    arg3: u64,
    _arg4: u64,
    _arg5: u64,
    _arg6: u64,
) -> SyscallResult {
    let syscall = SyscallNumber::from_u64(syscall_num)
        .ok_or(SyscallError::InvalidSyscall)?;

    match syscall {
        SyscallNumber::Write => sys_write(arg1, arg2, arg3),
        SyscallNumber::Read => sys_read(arg1, arg2, arg3),
        SyscallNumber::Exit => sys_exit(arg1),
        SyscallNumber::GetPid => sys_getpid(),
        SyscallNumber::Fork => Err(SyscallError::NotImplemented),
        SyscallNumber::Exec => Err(SyscallError::NotImplemented),
    }
}

/// sys_write: Write to a file descriptor
///
/// Arguments:
/// - fd: file descriptor (0=stdin, 1=stdout, 2=stderr)
/// - buf: pointer to buffer in user space
/// - len: number of bytes to write
///
/// Returns: number of bytes written, or error
fn sys_write(fd: u64, buf: u64, len: u64) -> SyscallResult {
    // Validate file descriptor
    if fd != STDOUT && fd != STDERR {
        return Err(SyscallError::InvalidFileDescriptor);
    }

    // Validate buffer pointer (basic check - should be more thorough)
    if buf == 0 || len == 0 {
        return Ok(0);
    }

    // For now, we'll just print to kernel console
    // In a real implementation, we'd validate the user buffer is mapped
    // and copy data safely from user space

    // Safety: We're trusting the user pointer for now
    // TODO: Add proper user space memory validation
    unsafe {
        let slice = core::slice::from_raw_parts(buf as *const u8, len as usize);

        // Try to convert to string and print
        if let Ok(s) = core::str::from_utf8(slice) {
            if fd == STDOUT {
                print!("{}", s);
            } else {
                serial_println!("stderr: {}", s);
            }
        } else {
            // If not valid UTF-8, print raw bytes
            for byte in slice {
                if fd == STDOUT {
                    print!("{}", *byte as char);
                } else {
                    serial_print!("{}", *byte as char);
                }
            }
        }
    }

    Ok(len)
}

/// sys_read: Read from a file descriptor
///
/// Arguments:
/// - fd: file descriptor (0=stdin, 1=stdout, 2=stderr)
/// - buf: pointer to buffer in user space
/// - len: maximum number of bytes to read
///
/// Returns: number of bytes read, or error
fn sys_read(_fd: u64, _buf: u64, _len: u64) -> SyscallResult {
    // TODO: Implement keyboard input buffering
    // For now, return 0 (EOF)
    Err(SyscallError::NotImplemented)
}

/// sys_exit: Terminate the current process
///
/// Arguments:
/// - code: exit code
///
/// This syscall does not return
fn sys_exit(code: u64) -> SyscallResult {
    println!("Process exiting with code: {}", code);
    serial_println!("Process exiting with code: {}", code);

    // TODO: Properly terminate the current process
    // For now, just mark it as zombie in the process manager
    // This is a placeholder until we have proper process cleanup

    // Return success (though this should not return in final implementation)
    Ok(0)
}

/// sys_getpid: Get current process ID
///
/// Returns: current process ID
fn sys_getpid() -> SyscallResult {
    // TODO: Get actual PID from process manager
    // For now, return a dummy PID
    Ok(1)
}
