# Relibc: The Redox C Library

Relibc is a POSIX-compliant C library written in Rust. It serves as the primary C library for Redox OS and can also be used on Linux as a safer alternative to GLibc or Musl.

## Why Rust for a C Library?

Standard C libraries are often the source of critical security vulnerabilities due to memory safety issues. By implementing the C library in Rust, we eliminate entire classes of bugs (like buffer overflows and use-after-free) while maintaining high performance.

## Architecture

Relibc is designed to be modular and target-independent:

- **`src/`**: The core implementation of C functions (stdio, stdlib, string, etc.).
- **`platform/`**: Platform-specific implementations (Redox, Linux).
- **`ld_so/`**: The dynamic linker, responsible for loading Shared Objects (`.so` files).
- **`redox-rt/`**: The Redox runtime, providing the interface between user-space and the microkernel.

## POSIX Compliance

Relibc aims for high POSIX compatibility to allow standard C programs to be ported easily. This includes:

- Comprehensive support for `pthreads`.
- Standard I/O (buffered and unbuffered).
- Networking (sockets, DNS resolution).
- Signal handling.

## Dynamic Linking

The dynamic linker in Relibc supports modern ELF features, including TLS (Thread Local Storage), lazy binding, and shared memory mapping.
