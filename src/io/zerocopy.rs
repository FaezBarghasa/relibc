
use crate::io::{self, Read, Write, Seek, SeekFrom, Error, ErrorKind, DEFAULT_BUF_SIZE};
use core::cmp;

pub struct ZeroCopyBufReader<R> {
    inner: R,
    buf: Box<[u8]>,
    pos: usize,
    cap: usize,
}

impl<R: Read> ZeroCopyBufReader<R> {
    pub fn new(inner: R) -> Self {
        Self::with_capacity(DEFAULT_BUF_SIZE, inner)
    }

    pub fn with_capacity(cap: usize, inner: R) -> Self {
        let mut buf = Vec::with_capacity(cap);
        unsafe {
            buf.set_len(cap);
        }
        Self {
            inner,
            buf: buf.into_boxed_slice(),
            pos: 0,
            cap: 0,
        }
    }

    fn fill_buf(&mut self) -> io::Result<&[u8]> {
        if self.pos >= self.cap {
            self.cap = self.inner.read(&mut self.buf)?;
            self.pos = 0;
        }
        Ok(&self.buf[self.pos..self.cap])
    }
}

impl<R: Read> Read for ZeroCopyBufReader<R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let nread = {
            let mut rem = self.fill_buf()?;
            rem.read(buf)?
        };
        self.pos += nread;
        Ok(nread)
    }
}

pub struct ZeroCopyBufWriter<W: Write> {
    inner: Option<W>,
    buf: Vec<u8>,
}

impl<W: Write> ZeroCopyBufWriter<W> {
    pub fn new(inner: W) -> Self {
        Self::with_capacity(DEFAULT_BUF_SIZE, inner)
    }

    pub fn with_capacity(capacity: usize, inner: W) -> Self {
        Self {
            inner: Some(inner),
            buf: Vec::with_capacity(capacity),
        }
    }

    fn flush_buf(&mut self) -> io::Result<()> {
        if !self.buf.is_empty() {
            let written = self.inner.as_mut().unwrap().write(&self.buf)?;
            if written < self.buf.len() {
                // This is a simplistic handling. A real implementation would need to handle partial writes.
                return Err(Error::new(ErrorKind::WriteZero, "failed to write entire buffer"));
            }
            self.buf.clear();
        }
        Ok(())
    }
}

impl<W: Write> Write for ZeroCopyBufWriter<W> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        if self.buf.len() + buf.len() > self.buf.capacity() {
            self.flush_buf()?;
        }

        if buf.len() >= self.buf.capacity() {
            self.inner.as_mut().unwrap().write(buf)
        } else {
            self.buf.extend_from_slice(buf);
            Ok(buf.len())
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        self.flush_buf()?;
        self.inner.as_mut().unwrap().flush()
    }
}

impl<W: Write> Drop for ZeroCopyBufWriter<W> {
    fn drop(&mut self) {
        if self.inner.is_some() {
            let _ = self.flush_buf();
        }
    }
}
