use encoding_rs::{DecoderResult, Encoding, UTF_8};

pub(crate) const MAX_CHARACTERS: usize = 100_000;
pub(crate) const MAX_BYTES: u64 = 400_000;
const MIN_HALVED_CHARACTERS: usize = 1_000;

pub(crate) fn is_utf16(start: &[u8]) -> bool {
    Encoding::for_bom(start).is_some_and(|(encoding, _)| encoding != UTF_8)
}

pub(crate) fn decode(bytes: &[u8], partial: bool) -> Option<String> {
    let mut decoder = UTF_8.new_decoder();
    let capacity = decoder.max_utf8_buffer_length_without_replacement(bytes.len())?;
    let mut text = String::with_capacity(capacity);
    let (result, _) = decoder.decode_to_string_without_replacement(bytes, &mut text, !partial);
    match result {
        DecoderResult::InputEmpty if !text.contains('\0') => Some(text),
        DecoderResult::InputEmpty | DecoderResult::OutputFull | DecoderResult::Malformed(..) => {
            None
        }
    }
}

pub(crate) fn shorten(text: &mut String) -> bool {
    shorten_to(text, MAX_CHARACTERS)
}

pub(crate) fn halve(text: &mut String) -> bool {
    let half = text.chars().count() / 2;
    half >= MIN_HALVED_CHARACTERS && shorten_to(text, half)
}

fn shorten_to(text: &mut String, characters: usize) -> bool {
    match text.char_indices().nth(characters) {
        Some((end, _)) => {
            text.truncate(end);
            true
        }
        None => false,
    }
}
