use std::io;
use std::path::Path;

use tokio::fs::{self, File};
use tokio::io::AsyncReadExt;

use crate::decision::{Outcome, Skip};
use crate::text;

const MAX_CHARACTERS: usize = 100_000;
const MAX_BYTES: u64 = 400_000;

pub(crate) struct Content {
    pub(crate) text: String,
    pub(crate) truncated: bool,
}

pub(crate) async fn read(path: &Path) -> Result<Content, Outcome> {
    let unreadable = |error| Outcome::unreadable(&error);
    let metadata = fs::metadata(path).await.map_err(unreadable)?;
    if metadata.is_dir() {
        return Err(Outcome::skipped(Skip::Directory));
    }
    let bytes = read_prefix(path).await.map_err(unreadable)?;
    if bytes.is_empty() {
        return Err(Outcome::skipped(Skip::Empty));
    }
    let partial = metadata.len() > MAX_BYTES;
    let mut text = text::decode(bytes, partial).ok_or(Outcome::skipped(Skip::Binary))?;
    let shortened = shorten(&mut text);
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

fn shorten(text: &mut String) -> bool {
    match text.char_indices().nth(MAX_CHARACTERS) {
        Some((end, _)) => {
            text.truncate(end);
            true
        }
        None => false,
    }
}
