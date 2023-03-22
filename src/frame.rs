use crate::error::ParseError;
use cursor::{byte, integer, line, size, slice};
use std::io::Cursor;
use std::str::from_utf8;

#[derive(Debug, PartialEq)]
pub enum Frame<'a> {
    Simple(&'a str),
    Error(&'a str),
    Integer(i64),
    Bulk(&'a [u8]),
    Array(Vec<Self>),
    Slice(&'a [Self]),
}

impl<'a> Frame<'a> {
    /// Check that the contents of `src` can be parsed into a valid [`Frame`],
    /// without actually building a `Frame`. This method avoids allocation,
    /// which is beneficial because we often expect that not all of a message
    /// may have been received yet.
    pub fn check(src: &mut Cursor<&[u8]>) -> Result<(), ParseError> {
        match byte(src)? {
            b'+' => {
                from_utf8(line(src)?)?;
            }
            b'-' => {
                from_utf8(line(src)?)?;
            }
            b':' => {
                integer(src)?;
            }
            b'$' => {
                let len = size(src)?;

                slice(src, len)?;

                if b"\r\n" != slice(src, 2)? {
                    return Err(ParseError::NonTerminatedBulk);
                }
            }
            b'*' => {
                let len = size(src)?;

                for _ in 0..len {
                    Self::check(src)?;
                }
            }
            actual => return Err(ParseError::FrameType(actual)),
        }
        Ok(())
    }

    /// Parses the contents of `src` into a [`Frame`].
    pub fn decode(src: &mut Cursor<&'a [u8]>) -> Result<Self, ParseError> {
        match byte(src)? {
            b'+' => {
                let line = line(src)?;
                let string = from_utf8(line)?;
                Ok(Self::Simple(string))
            }
            b'-' => {
                let line = line(src)?;
                let string = from_utf8(line)?;
                Ok(Self::Error(string))
            }
            b':' => {
                let decimal = integer(src)?;
                Ok(Self::Integer(decimal))
            }
            b'$' => {
                let len = size(src)?;

                let data = slice(src, len)?;

                if b"\r\n" != slice(src, 2)? {
                    return Err(ParseError::NonTerminatedBulk);
                }

                Ok(Self::Bulk(data))
            }
            b'*' => {
                let len = size(src)?;
                let mut out = Vec::with_capacity(len as usize);

                for _ in 0..len {
                    out.push(Self::decode(src)?);
                }

                Ok(Self::Array(out))
            }
            actual => Err(ParseError::FrameType(actual)),
        }
    }

    pub fn as_simple(&self) -> Option<&str> {
        match self {
            Self::Simple(string) => Some(string),
            _ => None,
        }
    }

    pub fn as_array(&self) -> Option<&[Self]> {
        match self {
            Self::Array(array) => Some(array),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    // TODO: write tests here
}
