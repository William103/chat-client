use crate::error::ReadError;
use crate::frame::Frame;
use readbuf::ReadBuf;
use std::io::{self, Cursor, Read, Write};

pub struct FrameReader<R> {
    reader: R,
    buf: ReadBuf,
}

impl<R: Read> FrameReader<R> {
    pub fn new(reader: R) -> Self {
        FrameReader {
            reader,
            buf: ReadBuf::new(),
        }
    }

    pub fn read_frame(&mut self) -> Result<Guard<'_>, ReadError> {
        self.buf.read(&mut self.reader)?;
        let mut cursor = Cursor::new(self.buf.buf());

        Frame::check(&mut cursor)?;

        let len = cursor.position() as usize;

        Ok(Guard {
            buf: &mut self.buf,
            len,
        })
    }
}

pub trait WriteFrame {
    fn write_frame(&mut self, frame: &Frame) -> io::Result<()>;
}

impl<W: Write> WriteFrame for W {
    fn write_frame(&mut self, frame: &Frame) -> io::Result<()> {
        match frame {
            Frame::Simple(val) => {
                write!(self, "+{val}\r\n")?;
            }
            Frame::Error(val) => {
                write!(self, "-{val}\r\n")?;
            }
            Frame::Integer(val) => {
                write!(self, ":{val}\r\n")?;
            }
            Frame::Bulk(bytes) => {
                write!(self, "${}\r\n", bytes.len())?;
                self.write_all(bytes)?;
                write!(self, "\r\n")?;
            }
            Frame::Array(array) => {
                write!(self, "*{}\r\n", array.len())?;
                for inner in array {
                    self.write_frame(inner)?;
                }
            }
            Frame::Slice(slice) => {
                write!(self, "*{}\r\n", slice.len())?;
                for inner in slice.iter() {
                    self.write_frame(inner)?;
                }
            }
        };

        Ok(())
    }
}

pub struct Guard<'a> {
    buf: &'a mut ReadBuf,
    len: usize,
}

impl Guard<'_> {
    pub fn frame(&self) -> Frame<'_> {
        Frame::decode(&mut Cursor::new(self.buf.buf())).expect("frame was checked")
    }
}

impl Drop for Guard<'_> {
    fn drop(&mut self) {
        self.buf.consume(self.len);
    }
}

#[cfg(test)]
mod tests {
    // TODO: tests
}
