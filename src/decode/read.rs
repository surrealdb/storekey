use std::io::{Read, BufRead, self, ErrorKind};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Reference<'b, 'c> {
    /// Borrowed from the buffer.
    Borrowed(&'b [u8]),
    /// Copied from the input onto the heap.
    Copied(&'c [u8]),
}

/// For zero-copy reading.
pub trait ReadReference<'de>: Read + BufRead {
    /// Reads the exact number of bytes from the underlying byte-array.
    fn read_reference_until<'a>(&'a mut self, delimiter: u8) -> Result<Reference<'de, 'a>, io::Error>;
}

#[derive(Debug)]
pub(crate) struct ReadReader<R: Read> {
    inner: R,
    buf: Vec<u8>,
}

impl<R: Read> ReadReader<R> {
    #[inline]
    pub(crate) fn new(inner: R) -> Self {
        ReadReader {
            inner,
            buf: Vec::with_capacity(128),
        }
    }
}

impl<'de, R: BufRead> ReadReference<'de> for ReadReader<R> {
    #[inline]
    fn read_reference_until<'a>(&'a mut self, delimiter: u8) -> Result<Reference<'de, 'a>, io::Error> {
        self.buf.clear();
        self.inner.read_until(delimiter, &mut self.buf)?;
        Ok(Reference::Copied(&self.buf[0..self.buf.len().saturating_sub(1)]))
    }
}

impl<R: Read> Read for ReadReader<R> {
    #[inline]
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.inner.read(buf)
    }

    #[inline]
    fn read_exact(&mut self, buf: &mut [u8]) -> io::Result<()> {
        self.inner.read_exact(buf)
    }
}

impl<R: BufRead> BufRead for ReadReader<R> {
    fn fill_buf(&mut self) -> io::Result<&[u8]> {
        self.inner.fill_buf()
    }

    fn consume(&mut self, amt: usize) {
        self.inner.consume(amt);
    }

    fn read_until(&mut self, byte: u8, buf: &mut Vec<u8>) -> io::Result<usize> {
        self.inner.read_until(byte, buf)
    }
}

/// Borrowed reader wrapper.
#[derive(Debug)]
pub(crate) struct SliceReader<'a> {
    /// Haven't read yet.
    inner: &'a [u8],
}

impl<'a> SliceReader<'a> {
    #[inline]
    pub fn new(inner: &'a [u8]) -> Self {
        Self {
            inner,
        }
    }
}

impl<'a> Read for SliceReader<'a> {
    #[inline]
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, io::Error> {
        self.inner.read(buf)
    }

    #[inline]
    fn read_exact(&mut self, buf: &mut [u8]) -> Result<(), io::Error> {
        self.inner.read_exact(buf)
    }
}

impl<'a> BufRead for SliceReader<'a> {
    fn fill_buf(&mut self) -> io::Result<&[u8]> {
        self.inner.fill_buf()
    }

    fn consume(&mut self, amt: usize) {
        self.inner.consume(amt);
    }
}

impl<'de> ReadReference<'de> for SliceReader<'de> {
    #[inline]
    fn read_reference_until<'a>(&'a mut self, delimiter: u8) -> Result<Reference<'de, 'a>, io::Error> {
        if let Some(end) = memchr::memchr(delimiter, self.inner) {
            let (before, after) = self.inner.split_at(end);
            self.inner = &after[1..];
            Ok(Reference::Borrowed(before))
        } else {
            Err(io::Error::new(ErrorKind::UnexpectedEof, "unexpected EOF"))
        }
    }
}