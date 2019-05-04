use failure::{Backtrace, Context, Fail};
use std::fmt::{self, Display};

#[derive(Debug)]
pub struct Error(Context<ErrorKind>);

#[derive(Copy, Clone, Eq, PartialEq, Debug, Fail)]
pub enum ErrorKind {
    #[fail(display = "Missing elm.json")]
    MissingElmJson,
    #[fail(display = "Invalid elm.json")]
    InvalidElmJson,
    #[fail(display = "Failed to write elm.json")]
    UnwritableElmJson,
    #[fail(display = "No valid package version")]
    NoResolution,
    #[fail(display = "Not yet implemented")]
    NotImplemented,
    #[fail(display = "Unknown error")]
    Unknown,
}

impl Fail for Error {
    fn name(&self) -> Option<&str> {
        self.0.name()
    }

    fn cause(&self) -> Option<&Fail> {
        self.0.cause()
    }

    fn backtrace(&self) -> Option<&Backtrace> {
        self.0.backtrace()
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        Display::fmt(&self.0, f)
    }
}

impl Error {
    pub fn kind(&self) -> ErrorKind {
        *self.0.get_context()
    }
}

impl From<ErrorKind> for Error {
    fn from(kind: ErrorKind) -> Self {
        Self(Context::new(kind))
    }
}

impl From<Context<ErrorKind>> for Error {
    fn from(inner: Context<ErrorKind>) -> Self {
        Self(inner)
    }
}
