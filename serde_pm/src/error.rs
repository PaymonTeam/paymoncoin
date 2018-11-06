use std::error;
use std::fmt::{self, Debug, Display};
use std::io;
use std::result;

use serde::de;
use serde::ser;

pub struct Error {
    /// This `Box` allows us to keep the size of `Error` as small as possible. A
    /// larger `Error` type was substantially slower due to all the functions
    /// that pass around `Result<T, Error>`.
    err: Box<SerializationError>,
}

/// Alias for a `Result` with the error type `serde_json::Error`.
pub type Result<T> = result::Result<T, Error>;

#[derive(Debug)]
pub enum SerializationError {
    BoolError,
    I8Error,
    U8Error,
    I32Error,
    U32Error,
    I64Error,
    U64Error,
    StringError,
    BytesError,
    ByteArrayError,
    WrongSVUID,
    LimitExceeded,
    Message(String),
}

impl fmt::Display for SerializationError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", (self as error::Error).description())
    }
}

impl error::Error for SerializationError {
    fn description(&self) -> &str {
        match *self {
            SerializationError::BoolError => "failed to (de)serialize bool",
            SerializationError::I8Error => "failed to (de)serialize i8",
            SerializationError::U8Error => "failed to (de)serialize u8",
            SerializationError::I32Error => "failed to (de)serialize i32",
            SerializationError::U32Error => "failed to (de)serialize u32",
            SerializationError::I64Error => "failed to (de)serialize i64",
            SerializationError::U64Error => "failed to (de)serialize u64",
            SerializationError::StringError => "failed to (de)serialize String",
            SerializationError::BytesError => "failed to (de)serialize bytes",
            SerializationError::ByteArrayError => "failed to (de)serialize byte array",
            SerializationError::WrongSVUID => "wrong SVUID",
            SerializationError::LimitExceeded => "buffer limit exceeded",
            SerializationError::Message(ref s) => s,
        }
    }
}

#[cfg_attr(feature = "cargo-clippy", allow(fallible_impl_from))]
impl From<Error> for io::Error {
    /// Convert a `serde_json::Error` into an `io::Error`.
    fn from(j: Error) -> Self {
        match *j.err {
            SerializationError::BoolError | SerializationError::I8Error  |
            SerializationError::I32Error | SerializationError::U32Error  |
            SerializationError::I64Error | SerializationError::U64Error  |
            SerializationError::StringError | SerializationError::BytesError  |
            SerializationError::ByteArrayError | SerializationError::U8Error => io::Error::new(io::ErrorKind::InvalidInput, j),
            SerializationError::WrongSVUID => io::Error::new(io::ErrorKind::NotFound, j),
            SerializationError::LimitExceeded => io::Error::new(io::ErrorKind::UnexpectedEof, j),
            SerializationError::Message(s) => io::Error::new(io::ErrorKind::Other, j),
        }
    }
}

impl de::Error for Error {
    #[cold]
    fn custom<T: Display>(msg: T) -> Error {
        Error {
            err: Box::new(SerializationError::Message(msg.to_string().into_boxed_str()))
        }
    }

    #[cold]
    fn invalid_type(unexp: de::Unexpected, exp: &de::Expected) -> Self {
        if let de::Unexpected::Unit = unexp {
            Error::custom(format_args!("invalid type: null, expected {}", exp))
        } else {
            Error::custom(format_args!("invalid type: {}, expected {}", unexp, exp))
        }
    }
}

impl ser::Error for Error {
    #[cold]
    fn custom<T: Display>(msg: T) -> Error {
        Error {
            err: Box::new(SerializationError::Message(msg.to_string().into_boxed_str()))
        }
    }
}
