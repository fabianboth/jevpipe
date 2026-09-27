use std::fmt;
use std::io;

#[derive(Clone, Copy)]
pub(crate) enum Skip {
    Directory,
    Empty,
    Binary,
}

#[derive(Debug)]
pub(crate) enum Failure {
    Unreadable(io::ErrorKind),
    NotText,
    Utf16,
    TooLarge,
    ServiceUnavailable,
}

impl From<io::Error> for Failure {
    fn from(error: io::Error) -> Self {
        Self::Unreadable(error.kind())
    }
}

impl fmt::Display for Skip {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Directory => "directory",
            Self::Empty => "empty",
            Self::Binary => "binary",
        })
    }
}

impl fmt::Display for Failure {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unreadable(kind) if *kind == io::ErrorKind::NotFound => {
                formatter.write_str("not found")
            }
            Self::Unreadable(kind) => write!(formatter, "{kind}"),
            Self::NotText => formatter.write_str("not text"),
            Self::Utf16 => formatter.write_str("UTF-16, convert it to UTF-8"),
            Self::TooLarge => formatter.write_str("too large"),
            Self::ServiceUnavailable => formatter.write_str("service unavailable"),
        }
    }
}
