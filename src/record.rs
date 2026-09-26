use std::fs::File;
use std::io::{self, BufRead, BufReader};
use std::ops::ControlFlow;
use std::path::{Path, PathBuf};
use std::thread;

use futures::Stream;
use tokio::sync::mpsc;

use crate::decision::{Decision, Failure, Outcome};
use crate::text;

const BUFFERED_RECORDS: usize = 256;
const STANDARD_INPUT: &str = "(standard input)";
const STANDARD_INPUT_ARGUMENT: &str = "-";

pub(crate) struct Record {
    pub(crate) position: usize,
    pub(crate) raw: Vec<u8>,
    pub(crate) text: String,
}

pub(crate) type Incoming = Result<Record, Decision>;

pub(crate) fn read(mut inputs: Vec<PathBuf>) -> io::Result<impl Stream<Item = Incoming>> {
    if inputs.is_empty() {
        inputs.push(PathBuf::from(STANDARD_INPUT_ARGUMENT));
    }
    let (sender, receiver) = mpsc::channel(BUFFERED_RECORDS);
    let reader = Reader {
        sender,
        position: 0,
    };
    thread::Builder::new()
        .name("input".to_owned())
        .spawn(move || reader.read_all(&inputs))?;
    Ok(futures::stream::unfold(receiver, |mut receiver| async {
        receiver.recv().await.map(|incoming| (incoming, receiver))
    }))
}

struct Reader {
    sender: mpsc::Sender<Incoming>,
    position: usize,
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
            Err(error) => self.send_failed(name, Outcome::unreadable(&error)),
        }
    }

    fn read_lines(&mut self, input_name: &str, mut reader: impl BufRead) -> ControlFlow<()> {
        let mut first_line = true;
        loop {
            let mut raw = Vec::new();
            match reader.read_until(b'\n', &mut raw) {
                Ok(0) => return ControlFlow::Continue(()),
                Ok(_) if first_line && text::is_utf16(&raw) => {
                    return self
                        .send_failed(input_name.to_owned(), Outcome::failed(Failure::Utf16));
                }
                Ok(_) if raw.trim_ascii().is_empty() => {}
                Ok(_) => {
                    let incoming = self.record(raw);
                    self.send(incoming)?;
                }
                Err(error) => {
                    return self.send_failed(input_name.to_owned(), Outcome::unreadable(&error));
                }
            }
            first_line = false;
        }
    }

    fn record(&mut self, raw: Vec<u8>) -> Incoming {
        let position = self.next_position();
        let line = without_terminator(&raw);
        let is_text = text::is_text(line);
        let text = String::from_utf8_lossy(line).into_owned();
        let record = Record {
            position,
            raw,
            text,
        };
        if is_text {
            return Ok(record);
        }
        Err(Decision {
            record,
            outcome: Outcome::failed(Failure::NotText),
        })
    }

    fn send_failed(&mut self, input_name: String, outcome: Outcome) -> ControlFlow<()> {
        let record = Record {
            position: self.next_position(),
            raw: Vec::new(),
            text: input_name,
        };
        self.send(Err(Decision { record, outcome }))
    }

    fn send(&self, incoming: Incoming) -> ControlFlow<()> {
        match self.sender.blocking_send(incoming) {
            Ok(()) => ControlFlow::Continue(()),
            Err(_) => ControlFlow::Break(()),
        }
    }

    fn next_position(&mut self) -> usize {
        self.position += 1;
        self.position
    }
}

fn without_terminator(raw: &[u8]) -> &[u8] {
    let line = raw.strip_suffix(b"\n").unwrap_or(raw);
    line.strip_suffix(b"\r").unwrap_or(line)
}
