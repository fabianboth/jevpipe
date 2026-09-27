use std::fmt::Display;
use std::process::ExitCode;

use crate::output::report;

pub(crate) enum Exit {
    Success,
    NothingKept,
    Error,
    Stopped,
    Interrupted,
}

pub(crate) fn fail(error: impl Display) -> ExitCode {
    report(format_args!("error: {error}"));
    Exit::Error.into()
}

impl From<Exit> for ExitCode {
    fn from(exit: Exit) -> Self {
        match exit {
            Exit::Success => Self::SUCCESS,
            Exit::NothingKept => Self::FAILURE,
            Exit::Error => Self::from(2),
            Exit::Stopped => Self::from(3),
            Exit::Interrupted => Self::from(130),
        }
    }
}
