#![forbid(unsafe_code)]

use crate::{
    c_str::{CStr, CString},
    error::{Errno, ResultExt},
    platform::{Pal, Sys, types::*},
};
use alloc::format;
use alloc::vec::Vec;
use zerocopy::{AsBytes, FromBytes, FromZeroes};

#[derive(AsBytes, FromBytes, FromZeroes, Clone, Copy, Debug)]
#[repr(C)]
pub struct BinderTarget {
    pub val: u64,
}

#[derive(AsBytes, FromBytes, FromZeroes, Clone, Copy, Debug)]
#[repr(C)]
pub struct BinderTransactionData {
    pub target: BinderTarget,
    pub cookie: u64,
    pub code: u32,
    pub flags: u32,
    pub sender_pid: i32,
    pub sender_euid: u32,
    pub data_size: u64,
    pub offsets_size: u64,
    pub data_buffer: u64,
    pub offsets_buffer: u64,
}

#[derive(AsBytes, FromBytes, FromZeroes, Clone, Copy, Debug)]
#[repr(C)]
pub struct BinderWriteRead {
    pub write_size: u64,
    pub write_consumed: u64,
    pub write_buffer: u64,
    pub read_size: u64,
    pub read_consumed: u64,
    pub read_buffer: u64,
}

pub struct BinderSchemeMapper {
    shm_fd: c_int,
    shm_path: Vec<u8>,
}

impl BinderSchemeMapper {
    pub fn new(id: usize) -> Result<Self, Errno> {
        let path_str = format!("/scheme/shm/binder-shm-{}", id);
        let path_c = match CString::new(path_str) {
            Ok(c) => c,
            Err(_) => return Err(Errno(crate::header::errno::EINVAL)),
        };
        let fd = Sys::open(
            CStr::borrow(&path_c),
            crate::header::fcntl::O_CREAT | crate::header::fcntl::O_RDWR,
            0o600,
        ).map_err(|e| Errno(e.0))?;

        let path_vec = path_c.into_bytes();
        Ok(Self {
            shm_fd: fd,
            shm_path: path_vec,
        })
    }

    pub fn set_size(&self, size: usize) -> Result<(), Errno> {
        Sys::ftruncate(self.shm_fd, size as off_t).map_err(|e| Errno(e.0))
    }

    pub fn path(&self) -> &[u8] {
        &self.shm_path
    }

    pub fn parse_transaction(&self, buf: &[u8]) -> Option<BinderTransactionData> {
        if buf.len() < core::mem::size_of::<BinderTransactionData>() {
            return None;
        }
        let mut data = BinderTransactionData::new_zeroed();
        data.as_bytes_mut().copy_from_slice(&buf[..core::mem::size_of::<BinderTransactionData>()]);
        Some(data)
    }

    pub fn serialize_transaction(&self, data: &BinderTransactionData) -> Vec<u8> {
        data.as_bytes().to_vec()
    }
}
