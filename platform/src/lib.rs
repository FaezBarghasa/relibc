#![no_std]

use core::fmt;

pub mod types;

pub trait Pal {
    fn write(fd: self::types::c_int, buf: &[u8]) -> self::types::ssize_t;
    fn read(fd: self::types::c_int, buf: &mut [u8]) -> self::types::ssize_t;
}

pub struct Sys;

impl Pal for Sys {
    fn write(fd: self::types::c_int, buf: &[u8]) -> self::types::ssize_t {
        // This is a placeholder. The actual implementation will be in the platform-specific code.
        0
    }
    fn read(fd: self::types::c_int, buf: &mut [u8]) -> self::types::ssize_t {
        // This is a placeholder. The actual implementation will be in the platform-specific code.
        0
    }
}

pub struct FileWriter(pub self::types::c_int, Option<self::types::c_int>);

impl FileWriter {
    pub fn new(fd: self::types::c_int) -> Self {
        Self(fd, None)
    }

    pub fn write(&mut self, buf: &[u8]) -> fmt::Result {
        let ret = Sys::write(self.0, buf);
        if ret < 0 {
            self.1 = Some(-ret as self::types::c_int);
            Err(fmt::Error)
        } else {
            Ok(())
        }
    }
}

impl fmt::Write for FileWriter {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.write(s.as_bytes())
    }
}
