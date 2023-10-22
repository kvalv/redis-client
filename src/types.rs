use std::fmt::Display;
use std::io;

pub type RedisResult<T> = std::result::Result<T, RedisError>;

#[derive(Debug, Clone)]
pub enum RedisError {
    IoError(io::ErrorKind),
    ErrorResponse(String),
    ErrIllegalTypeConversion,
    NoBytesWriten,
    ConnectionError,
    UnexpectedResponseType,
}

impl From<io::Error> for RedisError {
    fn from(value: io::Error) -> Self {
        RedisError::IoError(value.kind())
    }
}

impl Display for RedisError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "redis error: {}", self)
    }
}
