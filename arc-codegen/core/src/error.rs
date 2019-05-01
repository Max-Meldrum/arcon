use std::error::Error as StdError;
use std::fmt;

#[derive(Debug, Eq, PartialEq)]
pub enum ErrorKind {
    CompilationError(String),
    ModuleRunError(String),
    ContextError(String),
    SerializationError(String),
    BadTaskError(String),
    IOError(String),
}

#[derive(Debug)]
pub struct Error {
    kind: ErrorKind,
    cause: Option<Box<StdError + Send>>,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let msg = match self.kind {
            ErrorKind::CompilationError(ref err) => err,
            ErrorKind::ContextError(ref err) => err,
            ErrorKind::ModuleRunError(ref err) => err,
            ErrorKind::BadTaskError(ref err) => err,
            ErrorKind::SerializationError(ref err) => err,
            ErrorKind::IOError(ref err) => err,
        };
        write!(f, "{}", msg)
    }
}

impl StdError for Error {
    fn cause(&self) -> Option<&StdError> {
        match self.cause {
            Some(ref x) => Some(&**x),
            None => None,
        }
    }
}

impl Error {
    pub fn new(kind: ErrorKind) -> Self {
        Self { kind, cause: None }
    }

    pub fn with_cause<E>(kind: ErrorKind, cause: E) -> Self
    where
        E: 'static + Send + StdError,
    {
        Self {
            kind,
            cause: Some(Box::new(cause)),
        }
    }

    pub fn kind(&self) -> &ErrorKind {
        &self.kind
    }
}

pub type Result<T> = ::std::result::Result<T, Error>;
