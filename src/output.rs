use std::fmt::Display;
use std::io::{self, StdoutLock, Write};

pub(crate) struct Output {
    stdout: StdoutLock<'static>,
}

pub(crate) enum Delivery {
    Open,
    Closed,
}

impl Output {
    pub(crate) fn new() -> Self {
        Self {
            stdout: io::stdout().lock(),
        }
    }

    pub(crate) fn write(
        &mut self,
        write: impl FnOnce(&mut StdoutLock<'static>) -> io::Result<()>,
    ) -> io::Result<Delivery> {
        match write(&mut self.stdout).and_then(|()| self.stdout.flush()) {
            Ok(()) => Ok(Delivery::Open),
            Err(error) if error.kind() == io::ErrorKind::BrokenPipe => Ok(Delivery::Closed),
            Err(error) => Err(error),
        }
    }
}

pub(crate) fn report(message: impl Display) {
    let _ = writeln!(io::stderr(), "jevpipe: {message}");
}
