//UTF implementation parts for wchar.h.
//Partially ported from the Sortix libc

use core::{char, slice, str, usize};

use crate::{
    header::errno,
    platform::{self, types::*},
};

use super::mbstate_t;

// Based on
// https://github.com/rust-lang/rust/blob/f24ce9b/library/core/src/str/validations.rs#L232-L257,
// because apparently somebody removed the `pub use` statement from `core::str`.

// https://tools.ietf.org/html/rfc3629
static UTF8_CHAR_WIDTH: [u8; 256] = [
    1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
    1, // 0x1F
    1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
    1, // 0x3F
    1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
    1, // 0x5F
    1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
    1, // 0x7F
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, // 0x9F
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, // 0xBF
    0, 0, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2,
    2, // 0xDF
    3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, // 0xEF
    4, 4, 4, 4, 4, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, // 0xFF
];

// Given a first byte, determines how many bytes are in this UTF-8 character.
#[inline]
fn utf8_char_width(b: u8) -> usize {
    UTF8_CHAR_WIDTH[usize::from(b)].into()
}

//It's guaranteed that we don't have any nullpointers here
pub unsafe fn mbrtowc(pwc: *mut wchar_t, s: *const c_char, n: usize, ps: *mut mbstate_t) -> usize {
    // ps is guaranteed non-null by caller (mod.rs)
    let ps = &mut *ps;

    let mut count = ps.__count as usize;
    let mut buffer: [u8; 4] = (ps.__value as u32).to_le_bytes();
    let mut bytes_read_from_s = 0;

    if count == 0 {
        if n == 0 {
            return -2isize as usize;
        }
        // Start new sequence
        let b = *s as u8;
        buffer[0] = b;
        count = 1;
        bytes_read_from_s = 1;
    }

    // Determine expected width
    let width = utf8_char_width(buffer[0]);
    if width == 0 {
        platform::ERRNO.set(errno::EILSEQ);
        ps.__count = 0;
        return -1isize as usize;
    }

    // Read remaining bytes
    while count < width && bytes_read_from_s < n {
        buffer[count] = *s.add(bytes_read_from_s) as u8;
        count += 1;
        bytes_read_from_s += 1;
    }

    if count < width {
        // Incomplete
        ps.__count = count as c_int;
        ps.__value = u32::from_le_bytes(buffer) as c_int;
        return -2isize as usize;
    }

    // Full sequence available
    match str::from_utf8(&buffer[..width]) {
        Ok(s_slice) => {
            let c = s_slice.chars().next().unwrap();
            if !pwc.is_null() {
                *pwc = c as wchar_t;
            }
            ps.__count = 0;
            ps.__value = 0;
            if c == '\0' { 0 } else { bytes_read_from_s }
        }
        Err(_) => {
            platform::ERRNO.set(errno::EILSEQ);
            ps.__count = 0; // Reset on error
            -1isize as usize
        }
    }
}

//It's guaranteed that we don't have any nullpointers here
pub unsafe fn wcrtomb(s: *mut c_char, wc: wchar_t, _ps: *mut mbstate_t) -> usize {
    let dc = char::from_u32(wc as u32);

    if dc.is_none() {
        platform::ERRNO.set(errno::EILSEQ);
        return -1isize as usize;
    }

    let c = dc.unwrap();
    let size = c.len_utf8();
    let slice = slice::from_raw_parts_mut(s as *mut u8, size);

    c.encode_utf8(slice);

    size
}

/// Gets the encoded length of a character. It is used to recognize wide characters
pub fn get_char_encoded_length(first_byte: u8) -> Option<usize> {
    if first_byte >> 7 == 0 {
        Some(1)
    } else if first_byte >> 5 == 6 {
        Some(2)
    } else if first_byte >> 4 == 0xe {
        Some(3)
    } else if first_byte >> 3 == 0x1e {
        Some(4)
    } else {
        None
    }
}
