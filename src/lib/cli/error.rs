use thiserror::Error;

#[derive(Copy, Clone, Eq, PartialEq, Debug, Error)]
pub enum Kind {
    #[error("Missing elm.json")]
    MissingElmJson,
    #[error("Invalid elm.json")]
    InvalidElmJson,
    #[error("Failed to write elm.json")]
    UnwritableElmJson,
    #[error("No valid package version")]
    NoResolution,
    #[error("Not supported")]
    NotSupported,
    #[error("Unknown error")]
    Unknown,
}

// impl Fail for Error {
//     fn name(&self) -> Option<&str> {
//         self.0.name()
//     }

//     fn cause(&self) -> Option<&dyn Fail> {
//         self.0.cause()
//     }

//     fn backtrace(&self) -> Option<&Backtrace> {
//         self.0.backtrace()
//     }
// }


// impl Error {
//     pub fn kind(&self) -> Kind {
//         *self.0.get_context()
//     }
// }

// impl From<Kind> for Error {
//     fn from(kind: Kind) -> Self {
//         Self(Context::new(kind))
//     }
// }

// impl From<Context<Kind>> for Error {
//     fn from(inner: Context<Kind>) -> Self {
//         Self(inner)
//     }
// }
