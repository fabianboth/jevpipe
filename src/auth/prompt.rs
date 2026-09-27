use std::io::{self, BufRead, IsTerminal, Write};
use std::thread;

use tokio::signal;
use tokio::sync::oneshot;

const PROMPT: &str = "OpenRouter API key: ";
const VISIBLE_PROMPT: &str = "OpenRouter API key (input will be visible): ";

pub(super) enum Typed {
    Line(String),
    Interrupted,
}

enum Terminal {
    Hidden { _echo_off: echo::Hidden },
    Visible,
    Piped,
}

pub(super) async fn read_line() -> io::Result<Typed> {
    let terminal = Terminal::open();
    terminal.prompt()?;
    let typed = read_or_interrupt().await?;
    terminal.finish(&typed)?;
    Ok(typed)
}

impl Terminal {
    fn open() -> Self {
        if !io::stdin().is_terminal() {
            return Self::Piped;
        }
        match echo::Hidden::start() {
            Ok(echo_off) => Self::Hidden {
                _echo_off: echo_off,
            },
            Err(_) => Self::Visible,
        }
    }

    fn prompt(&self) -> io::Result<()> {
        let prompt = match self {
            Self::Hidden { .. } => PROMPT,
            Self::Visible => VISIBLE_PROMPT,
            Self::Piped => return Ok(()),
        };
        let mut stderr = io::stderr();
        stderr.write_all(prompt.as_bytes())?;
        stderr.flush()
    }

    fn finish(self, typed: &Typed) -> io::Result<()> {
        match (self, typed) {
            (Self::Hidden { .. }, _) | (Self::Visible, Typed::Interrupted) => {
                writeln!(io::stderr())
            }
            (Self::Visible, Typed::Line(_)) | (Self::Piped, _) => Ok(()),
        }
    }
}

async fn read_or_interrupt() -> io::Result<Typed> {
    let (sender, receiver) = oneshot::channel();
    thread::Builder::new()
        .name("key".to_owned())
        .spawn(move || sender.send(read_one_line()))?;
    tokio::select! {
        line = receiver => line.map_err(io::Error::other)?.map(Typed::Line),
        interrupted = signal::ctrl_c() => interrupted.map(|()| Typed::Interrupted),
    }
}

fn read_one_line() -> io::Result<String> {
    let mut line = String::new();
    io::stdin().lock().read_line(&mut line)?;
    Ok(line)
}

#[cfg(windows)]
mod echo {
    use std::io;

    use winapi_util::HandleRef;
    use winapi_util::console::{mode, set_mode};

    const ENABLE_ECHO_INPUT: u32 = 0x0004;

    pub(super) struct Hidden {
        original: u32,
    }

    impl Hidden {
        pub(super) fn start() -> io::Result<Self> {
            let original = mode(HandleRef::stdin())?;
            set_mode(HandleRef::stdin(), original & !ENABLE_ECHO_INPUT)?;
            Ok(Self { original })
        }
    }

    impl Drop for Hidden {
        fn drop(&mut self) {
            let _ = set_mode(HandleRef::stdin(), self.original);
        }
    }
}

#[cfg(unix)]
mod echo {
    use std::io;

    use rustix::stdio::stdin;
    use rustix::termios::{LocalModes, OptionalActions, Termios, tcgetattr, tcsetattr};

    pub(super) struct Hidden {
        original: Termios,
    }

    impl Hidden {
        pub(super) fn start() -> io::Result<Self> {
            let original = tcgetattr(stdin())?;
            let mut hidden = original.clone();
            hidden.local_modes.remove(LocalModes::ECHO);
            tcsetattr(stdin(), OptionalActions::Now, &hidden)?;
            Ok(Self { original })
        }
    }

    impl Drop for Hidden {
        fn drop(&mut self) {
            let _ = tcsetattr(stdin(), OptionalActions::Now, &self.original);
        }
    }
}
