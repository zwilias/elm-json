use failure::{Backtrace, Context, Fail};
use std::fmt::{self, Display};

#[derive(Debug)]
pub struct Error(Context<Kind>);

#[derive(Copy, Clone, Eq, PartialEq, Debug, Fail)]
pub enum Kind {
    #[fail(display = "Missing elm.json")]
    MissingElmJson,
    #[fail(display = "Invalid elm.json")]
    InvalidElmJson,
    #[fail(display = "Failed to write elm.json")]
    UnwritableElmJson,
    #[fail(display = "No valid package version")]
    NoResolution,
    #[fail(display = "Not supported")]
    NotSupported,
    #[fail(display = "Unknown error")]
    Unknown,
}

impl Fail for Error {
    fn name(&self) -> Option<&str> {
        self.0.name()
    }

    fn cause(&self) -> Option<&dyn Fail> {
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
    pub fn kind(&self) -> Kind {
        *self.0.get_context()
    }
}

impl From<Kind> for Error {
    fn from(kind: Kind) -> Self {
        Self(Context::new(kind))
    }
}

impl From<Context<Kind>> for Error {
    fn from(inner: Context<Kind>) -> Self {
        Self(inner)
    }
}
