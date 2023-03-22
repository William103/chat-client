use cursor::CursorError;
use std::{io, str::Utf8Error};

#[derive(Debug, PartialEq, Eq)]
pub enum ParseError {
    Cursor(CursorError),
    FrameType(u8),
    Utf8,
    NonTerminatedBulk,
}

impl From<Utf8Error> for ParseError {
    fn from(_: Utf8Error) -> Self {
        Self::Utf8
    }
}

impl From<CursorError> for ParseError {
    fn from(err: CursorError) -> Self {
        Self::Cursor(err)
    }
}

#[derive(Debug)]
pub enum ReadError {
    /// Problem decoding
    Parse(ParseError),
    /// Io problem
    Io(io::Error),
}

// new in lab6
impl ReadError {
    pub fn is_pending(&self) -> bool {
        match self {
            Self::Parse(ParseError::Cursor(err)) => err.not_enough_data(),
            Self::Io(err) => err.kind() == io::ErrorKind::WouldBlock,
            _ => false,
        }
    }

    pub fn is_exhausted(&self) -> bool {
        matches!(self, Self::Io(err) if err.kind() == io::ErrorKind::WriteZero)
    }
}

impl From<io::Error> for ReadError {
    fn from(value: io::Error) -> Self {
        Self::Io(value)
    }
}

impl From<ParseError> for ReadError {
    fn from(value: ParseError) -> Self {
        Self::Parse(value)
    }
}
