use std::fs::File;
use std::io::{self, BufRead, BufReader};
use std::ops::ControlFlow;
use std::path::{Path, PathBuf};
use std::thread;

use futures::Stream;
use tokio::sync::mpsc;

use crate::reason::Failure;
use crate::text;

const BUFFERED_INPUTS: usize = 256;
const STANDARD_INPUT: &str = "(standard input)";
const STANDARD_INPUT_ARGUMENT: &str = "-";

pub(crate) enum Input {
    Record(Record),
    NotText(Record),
    Failed(FailedInput),
}

pub(crate) struct Record {
    pub(crate) line: usize,
    pub(crate) text: String,
    pub(crate) ending: Ending,
}

#[derive(Clone, Copy)]
pub(crate) enum Ending {
    Lf,
    CrLf,
    Missing,
}

pub(crate) struct FailedInput {
    pub(crate) name: String,
    pub(crate) failure: Failure,
}

pub(crate) fn read(mut inputs: Vec<PathBuf>) -> io::Result<impl Stream<Item = Input>> {
    if inputs.is_empty() {
        inputs.push(PathBuf::from(STANDARD_INPUT_ARGUMENT));
    }
    let (sender, receiver) = mpsc::channel(BUFFERED_INPUTS);
    let reader = Reader { sender, line: 0 };
    thread::Builder::new()
        .name("input".to_owned())
        .spawn(move || reader.read_all(&inputs))?;
    Ok(futures::stream::unfold(receiver, |mut receiver| async {
        receiver.recv().await.map(|input| (input, receiver))
    }))
}

struct Reader {
    sender: mpsc::Sender<Input>,
    line: usize,
}

impl Reader {
    fn read_all(mut self, inputs: &[PathBuf]) {
        for input in inputs {
            if self.read_input(input).is_break() {
                return;
            }
        }
    }

    fn read_input(&mut self, input: &Path) -> ControlFlow<()> {
        if input == Path::new(STANDARD_INPUT_ARGUMENT) {
            return self.read_lines(STANDARD_INPUT, io::stdin().lock());
        }
        let name = input.display().to_string();
        match File::open(input) {
            Ok(file) => self.read_lines(&name, BufReader::new(file)),
            Err(error) => self.send_failed(&name, error.into()),
        }
    }

    fn read_lines(&mut self, name: &str, mut reader: impl BufRead) -> ControlFlow<()> {
        let mut first_line = true;
        loop {
            let mut raw = Vec::new();
            match reader.read_until(b'\n', &mut raw) {
                Ok(0) => return ControlFlow::Continue(()),
                Ok(_) if first_line && text::is_utf16(&raw) => {
                    return self.send_failed(name, Failure::Utf16);
                }
                Ok(_) => {
                    self.line += 1;
                    if !raw.trim_ascii().is_empty() {
                        self.send(record(self.line, raw))?;
                    }
                }
                Err(error) => return self.send_failed(name, error.into()),
            }
            first_line = false;
        }
    }

    fn send_failed(&self, name: &str, failure: Failure) -> ControlFlow<()> {
        self.send(Input::Failed(FailedInput {
            name: name.to_owned(),
            failure,
        }))
    }

    fn send(&self, input: Input) -> ControlFlow<()> {
        match self.sender.blocking_send(input) {
            Ok(()) => ControlFlow::Continue(()),
            Err(_) => ControlFlow::Break(()),
        }
    }
}

fn record(line: usize, mut raw: Vec<u8>) -> Input {
    let ending = Ending::strip(&mut raw);
    let text = String::from_utf8(raw)
        .map_err(|error| String::from_utf8_lossy(error.as_bytes()).into_owned());
    match text {
        Ok(text) if !text.contains('\0') => Input::Record(Record { line, text, ending }),
        Ok(text) | Err(text) => Input::NotText(Record { line, text, ending }),
    }
}

impl Ending {
    fn strip(raw: &mut Vec<u8>) -> Self {
        let line_feed = raw.pop_if(|byte| *byte == b'\n').is_some();
        let carriage_return = raw.pop_if(|byte| *byte == b'\r').is_some();
        match (line_feed, carriage_return) {
            (_, true) => Self::CrLf,
            (true, false) => Self::Lf,
            (false, false) => Self::Missing,
        }
    }

    pub(crate) fn terminator(self) -> &'static [u8] {
        match self {
            Self::Lf | Self::Missing => b"\n",
            Self::CrLf => b"\r\n",
        }
    }
}
