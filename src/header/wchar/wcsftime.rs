use super::{wcrtomb, WriteWchar};

struct LocalCountingWriter<'a, W: ?Sized> {
    inner: &'a mut W,
    written: usize,
}

impl<'a, W: WriteWchar + ?Sized> LocalCountingWriter<'a, W> {
    fn new(inner: &'a mut W) -> Self {
        Self { inner, written: 0 }
    }
}

impl<'a, W: WriteWchar + ?Sized> WriteWchar for LocalCountingWriter<'a, W> {
    fn write_wchar(&mut self, c: wchar_t) -> crate::io::Result<()> {
        self.inner.write_wchar(c)?;
        self.written += 1;
        Ok(())
    }
}

use crate::{
    c_str::CStr,
    header::{
        langinfo::{nl_item, nl_langinfo, ABDAY_1, ABMON_1, AM_STR, DAY_1, MON_1, PM_STR},
        stdlib::MB_CUR_MAX,
        time::tm,
    },
    platform::{self, types::*},
};
use alloc::{string::String, vec::Vec};

unsafe fn langinfo_to_wcs(item: nl_item, w: &mut dyn WriteWchar) -> bool {
    let ptr = nl_langinfo(item);
    if ptr.is_null() {
        return true;
    }
    let c_str = CStr::from_ptr(ptr);
    for &byte in c_str.to_bytes() {
        if w.write_wchar(byte as wchar_t).is_err() {
            return false;
        }
    }
    true
}

pub unsafe fn wcsftime<W: WriteWchar>(w: &mut W, format: *const wchar_t, t: *const tm) -> size_t {
    pub unsafe fn inner_wcsftime<W: WriteWchar>(
        w: &mut W,
        mut format: *const wchar_t,
        t: *const tm,
    ) -> bool {
        macro_rules! w {
            (char $chr:expr) => {{
                if w.write_wchar($chr as wchar_t).is_err() {
                    return false;
                }
            }};
            (wstr $wstr:expr) => {{
                if w.write_wstr($wstr).is_err() {
                    return false;
                }
            }};
            (recurse $fmt:expr) => {{
                let mut tmp = String::with_capacity($fmt.len() + 1);
                tmp.push_str($fmt);
                tmp.push('\0');

                let mut wide_tmp = tmp.chars().map(|c| c as wchar_t).collect::<::alloc::vec::Vec<wchar_t>>();
                wide_tmp.push(0);

                if !inner_wcsftime(w, wide_tmp.as_ptr(), t) {
                    return false;
                }
            }};
            ($str:expr) => {{
                for c in $str.chars() {
                    if w.write_wchar(c as wchar_t).is_err() {
                        return false;
                    }
                }
            }};
            ($fmt:expr, $($args:expr),+) => {{
                let s = alloc::format!($fmt, $($args),+);
                for c in s.chars() {
                    if w.write_wchar(c as wchar_t).is_err() {
                        return false;
                    }
                }
            }};
        }

        while *format != 0 {
            if *format != '%' as wchar_t {
                w!(char * format);
                format = format.offset(1);
                continue;
            }

            format = format.offset(1);

            let modifier = if *format == 'E' as wchar_t || *format == 'O' as wchar_t {
                let m = *format;
                format = format.offset(1);
                m
            } else {
                0
            };

            match *format as u32 {
                // %%
                0x25 => w!(char '%'),

                // %n
                0x6E => w!(char '\n'),
                // %t
                0x74 => w!(char '\t'),

                // %a
                0x61 => {
                    if !langinfo_to_wcs(ABDAY_1 + (*t).tm_wday as i32, w) {
                        return false;
                    }
                }

                // %A
                0x41 => {
                    if !langinfo_to_wcs(DAY_1 + (*t).tm_wday as i32, w) {
                        return false;
                    }
                }

                // %b
                0x62 => {
                    if !langinfo_to_wcs(ABMON_1 + (*t).tm_mon as i32, w) {
                        return false;
                    }
                }

                // %B
                0x42 => {
                    if !langinfo_to_wcs(MON_1 + (*t).tm_mon as i32, w) {
                        return false;
                    }
                }

                // %c
                0x63 => {
                    let fmt = if modifier == 'E' as wchar_t {
                        nl_langinfo(crate::header::langinfo::ERA_D_T_FMT)
                    } else {
                        nl_langinfo(crate::header::langinfo::D_T_FMT)
                    };
                    if !fmt.is_null() {
                        let c_str = CStr::from_ptr(fmt);
                        let mut wide_tmp = c_str
                            .to_bytes()
                            .iter()
                            .map(|&c| c as wchar_t)
                            .collect::<Vec<wchar_t>>();
                        wide_tmp.push(0);
                        if !inner_wcsftime(w, wide_tmp.as_ptr(), t) {
                            return false;
                        }
                    } else {
                        w!(recurse "%a %b %e %T %Y");
                    }
                }

                // %C
                0x43 => {
                    let mut year = (*t).tm_year / 100;
                    if (*t).tm_year % 100 != 0 {
                        year += 1;
                    }
                    w!("{:02}", year + 19);
                }

                // %d
                0x64 => w!("{:02}", (*t).tm_mday),

                // %D
                0x44 => w!(recurse "%m/%d/%y"),

                // %e
                0x65 => w!("{:2}", (*t).tm_mday),

                // %F
                0x46 => w!(recurse "%Y-%m-%d"),

                // %H
                0x48 => w!("{:02}", (*t).tm_hour),

                // %I
                0x49 => w!("{:02}", ((*t).tm_hour + 12 - 1) % 12 + 1),

                // %j
                0x6A => w!("{:03}", (*t).tm_yday + 1),

                // %k
                0x6B => w!("{:2}", (*t).tm_hour),
                // %l
                0x6C => w!("{:2}", ((*t).tm_hour + 12 - 1) % 12 + 1),
                // %m
                0x6D => w!("{:02}", (*t).tm_mon + 1),
                // %M
                0x4D => w!("{:02}", (*t).tm_min),

                // %p
                0x70 => {
                    if (*t).tm_hour < 12 {
                        if !langinfo_to_wcs(AM_STR, w) {
                            return false;
                        }
                    } else {
                        if !langinfo_to_wcs(PM_STR, w) {
                            return false;
                        }
                    }
                }

                // %P
                0x50 => {
                    let item = if (*t).tm_hour < 12 { AM_STR } else { PM_STR };
                    let ptr = nl_langinfo(item);
                    if !ptr.is_null() {
                        let c_str = CStr::from_ptr(ptr);
                        for &byte in c_str.to_bytes() {
                            if w.write_wchar((byte as char).to_ascii_lowercase() as wchar_t)
                                .is_err()
                            {
                                return false;
                            }
                        }
                    }
                }

                // %r
                0x72 => w!(recurse "%I:%M:%S %p"),

                // %R
                0x52 => w!(recurse "%H:%M"),

                // %s
                0x73 => w!("{}", crate::header::time::mktime(t as *mut tm)),

                // %S
                0x53 => w!("{:02}", (*t).tm_sec),

                // %T
                0x54 => w!(recurse "%H:%M:%S"),

                // %u
                0x75 => w!("{}", ((*t).tm_wday + 7 - 1) % 7 + 1),

                // %U
                0x55 => w!("{}", ((*t).tm_yday + 7 - (*t).tm_wday) / 7),

                // %V
                0x56 => w!("{}", week_of_year(unsafe { &*t })),

                // %w
                0x77 => w!("{}", (*t).tm_wday),

                // %W
                0x57 => w!("{}", ((*t).tm_yday + 7 - ((*t).tm_wday + 6) % 7) / 7),

                // %x
                0x78 => {
                    let fmt = if modifier == 'E' as wchar_t {
                        nl_langinfo(crate::header::langinfo::ERA_D_FMT)
                    } else {
                        nl_langinfo(crate::header::langinfo::D_FMT)
                    };
                    if !fmt.is_null() {
                        let c_str = CStr::from_ptr(fmt);
                        let mut wide_tmp = c_str
                            .to_bytes()
                            .iter()
                            .map(|&c| c as wchar_t)
                            .collect::<Vec<wchar_t>>();
                        wide_tmp.push(0);
                        if !inner_wcsftime(w, wide_tmp.as_ptr(), t) {
                            return false;
                        }
                    } else {
                        w!(recurse "%m/%d/%y");
                    }
                }

                // %X
                0x58 => {
                    let fmt = if modifier == 'E' as wchar_t {
                        nl_langinfo(crate::header::langinfo::ERA_T_FMT)
                    } else {
                        nl_langinfo(crate::header::langinfo::T_FMT)
                    };
                    if !fmt.is_null() {
                        let c_str = CStr::from_ptr(fmt);
                        let mut wide_tmp = c_str
                            .to_bytes()
                            .iter()
                            .map(|&c| c as wchar_t)
                            .collect::<Vec<wchar_t>>();
                        wide_tmp.push(0);
                        if !inner_wcsftime(w, wide_tmp.as_ptr(), t) {
                            return false;
                        }
                    } else {
                        w!(recurse "%H:%M:%S");
                    }
                }

                // %y
                0x79 => w!("{:02}", (*t).tm_year % 100),

                // %Y
                0x59 => w!("{}", (*t).tm_year + 1900),

                // %z
                0x7A => {
                    let offset = (*t).tm_gmtoff;
                    let (sign, offset) = if offset < 0 {
                        ('-', -offset)
                    } else {
                        ('+', offset)
                    };
                    let mins = offset.div_euclid(60);
                    let min = mins.rem_euclid(60);
                    let hour = mins.div_euclid(60);
                    w!("{}{:02}{:02}", sign, hour, min)
                }

                // %Z
                0x5A => {
                    let s = CStr::from_ptr((*t).tm_zone).to_str().unwrap();
                    for c in s.chars() {
                        if w.write_wchar(c as wchar_t).is_err() {
                            return false;
                        }
                    }
                }

                // %+
                0x2B => w!(recurse "%a %b %d %T %Z %Y"),

                _ => return false,
            }

            format = format.offset(1);
        }
        true
    }

    let mut cw = LocalCountingWriter::new(w);
    if !inner_wcsftime(&mut cw, format, t) {
        return 0;
    }
    cw.written
}

fn weeks_per_year(year: c_int) -> c_int {
    let year = year as f64;
    let p_y = (year + (year / 4.) - (year / 100.) + (year / 400.)) as c_int % 7;
    if p_y == 4 {
        53
    } else {
        52
    }
}

fn week_of_year(time: &tm) -> c_int {
    let week = (10 + time.tm_yday - time.tm_wday) / 7;

    if week <= 1 {
        weeks_per_year(time.tm_year - 1)
    } else if week > weeks_per_year(time.tm_year) {
        1
    } else {
        week
    }
}
