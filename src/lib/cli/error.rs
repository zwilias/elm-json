use failure::{Backtrace, Context, Fail};
use std::fmt::{self, Display};

#[derive(Debug)]
pub struct Error {
    inner: Context<ErrorKind>,
}

#[derive(Copy, Clone, Eq, PartialEq, Debug, Fail)]
pub enum ErrorKind {
    #[fail(display = "MISSING ELM.JSON")]
    MissingElmJson,
    #[fail(display = "INVALID ELM.JSON")]
    InvalidElmJson,
    #[fail(display = "FAILED TO WRITE ELM.JSON")]
    UnwritableElmJson,
    #[fail(display = "NO VALID PACKAGE VERSIONS")]
    NoResolution,
    #[fail(display = "NOT YET IMPLEMENTED")]
    NotImplemented,
    #[fail(display = "UNKNOWN ERROR")]
    Unknown,
}

impl Fail for Error {
    fn name(&self) -> Option<&str> {
        self.inner.name()
    }

    fn cause(&self) -> Option<&Fail> {
        self.inner.cause()
    }

    fn backtrace(&self) -> Option<&Backtrace> {
        self.inner.backtrace()
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        Display::fmt(&self.inner, f)
    }
}

impl Error {
    pub fn kind(&self) -> ErrorKind {
        *self.inner.get_context()
    }
}

impl From<ErrorKind> for Error {
    fn from(kind: ErrorKind) -> Self {
        Self {
            inner: Context::new(kind),
        }
    }
}

impl From<Context<ErrorKind>> for Error {
    fn from(inner: Context<ErrorKind>) -> Self {
        Self { inner }
    }
}
