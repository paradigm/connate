//! Minimum platform-specific assembly code needed to satisfy ABI requirements.
//!
//! This should only be `use`'d by nostd, nolibc binaries:
//! - ./connate
//! - ./conctl
//!
//! This should *not* be `use`'d by:
//! - ./config
//! - ./build
//! - ./tests

#[cfg(not(any(
    all(target_os = "linux", target_arch = "aarch64"),
    all(target_os = "linux", target_arch = "x86_64"),
)))]
compile_error!("src/libc_shim/startup.rs only supports Linux x86_64 and Linux AArch64.");

#[cfg(not(test))]
#[cfg(all(target_os = "linux", target_arch = "aarch64"))]
core::arch::global_asm!(
    "
    .globl _start
_start:
    mov x9, sp
    ldr x0, [x9]             // argc
    add x1, x9, #8           // argv
    add x2, x1, x0, lsl #3   // envp = argv + argc * 8
    add x2, x2, #8           // skip null terminator after argv
    and sp, x9, #0xfffffffffffffff0 // align sp
    bl main
    mov x8, #93              // __NR_exit
    svc #0
"
);

#[cfg(not(test))]
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
core::arch::global_asm!(
    "
    .globl _start
    .type _start, @function
_start:
    mov rbp, rsp
    mov rdi, [rbp]                       # argc
    lea rsi, [rbp + 8]                   # argv
    mov rdx, rdi
    lea rdx, [rsi + rdx*8 + 8]           # envp pointer
    and rsp, 0xFFFFFFFFFFFFFFF0
    sub rsp, 8
    mov DWORD PTR [rsp], 0x00001F80
    ldmxcsr [rsp]
    mov WORD PTR [rsp], 0x037F
    fldcw [rsp]
    add rsp, 8
    call main
    mov rdi, rax
    mov rax, 60
    syscall
    .size _start, . - _start
"
);

#[allow(dead_code)]
#[cfg(not(test))]
unsafe extern "C" {
    fn main(
        argc: isize,
        argv: *const *const core::ffi::c_char,
        envp: *const *const core::ffi::c_char,
    ) -> core::ffi::c_int;
}
