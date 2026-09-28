use std::io;
use std::path::Path;

use tokio::fs::{self, File};
use tokio::io::AsyncReadExt;

use crate::reason::{Failure, Skip};
use crate::text::{self, MAX_BYTES};

pub(crate) struct Content {
    pub(crate) text: String,
    pub(crate) truncated: bool,
}

impl Content {
    pub(crate) fn halve(&mut self) -> bool {
        let halved = text::halve(&mut self.text);
        self.truncated |= halved;
        halved
    }
}

pub(crate) enum Unjudged {
    Skipped(Skip),
    Failed(Failure),
}

pub(crate) async fn read(path: &Path) -> Result<Content, Unjudged> {
    let metadata = fs::metadata(path).await?;
    if metadata.is_dir() {
        return Err(Unjudged::Skipped(Skip::Directory));
    }
    let bytes = read_prefix(path).await?;
    let partial = metadata.len() > MAX_BYTES;
    let mut text = text::decode(&bytes, partial).ok_or(Unjudged::Skipped(Skip::Binary))?;
    if text.is_empty() {
        return Err(Unjudged::Skipped(Skip::Empty));
    }
    let shortened = text::shorten(&mut text);
    Ok(Content {
        text,
        truncated: partial || shortened,
    })
}

async fn read_prefix(path: &Path) -> io::Result<Vec<u8>> {
    let mut bytes = Vec::new();
    File::open(path)
        .await?
        .take(MAX_BYTES)
        .read_to_end(&mut bytes)
        .await?;
    Ok(bytes)
}

impl From<io::Error> for Unjudged {
    fn from(error: io::Error) -> Self {
        Self::Failed(error.into())
    }
}
