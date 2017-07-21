use capnp;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ErrorKind {
    Encoding,
    Decoding,
    IO,
    Other,
    NotInSchema,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Error {
    kind: ErrorKind,
    description: String
}

pub fn decoding(s: &str) -> Error {
    Error{kind: ErrorKind::Decoding, description: format!("{}", s)}
}

pub fn encoding(s: &str) -> Error {
    Error{kind: ErrorKind::Decoding, description: format!("{}", s)}
}

use std::error;
impl error::Error for Error {
    fn description(&self) -> &str { self.description.as_str() }
}

use std::fmt;
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.description)
    }
}

impl From<capnp::Error> for Error {
    fn from(err: capnp::Error) -> Self {
        Error { kind: ErrorKind::Encoding, description: err.description }
    }
}


use std::error::Error as ErrTrait;
impl From<capnp::NotInSchema> for Error {
    fn from(err: capnp::NotInSchema) -> Self {
        Error { kind: ErrorKind::NotInSchema, description: err.description().to_owned()}
    }
}

use std::io;
impl From<io::Error> for Error {
    fn from(err: io::Error) -> Self {
        Error { kind: ErrorKind::NotInSchema, description: err.description().to_owned()}
    }
}

impl Into<io::Error> for Error {
    fn into(self) -> io::Error {
        io::Error::new(io::ErrorKind::Other, self)
    }
}

