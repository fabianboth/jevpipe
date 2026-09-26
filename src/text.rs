use std::ops::Range;

const UTF16_LE_BOM: [u8; 2] = [0xFF, 0xFE];
const UTF16_BE_BOM: [u8; 2] = [0xFE, 0xFF];
const HIGH_SURROGATES: Range<u16> = 0xD800..0xDC00;
const BINARY_PROBE_BYTES: usize = 8 * 1024;

pub(crate) fn is_utf16(start: &[u8]) -> bool {
    start.starts_with(&UTF16_LE_BOM) || start.starts_with(&UTF16_BE_BOM)
}

pub(crate) fn is_text(bytes: &[u8]) -> bool {
    !bytes.contains(&0) && str::from_utf8(bytes).is_ok()
}

pub(crate) fn decode(bytes: Vec<u8>, partial: bool) -> Option<String> {
    match bytes.split_first_chunk::<2>() {
        Some((&UTF16_LE_BOM, body)) => decode_utf16(body, u16::from_le_bytes, partial),
        Some((&UTF16_BE_BOM, body)) => decode_utf16(body, u16::from_be_bytes, partial),
        Some(_) | None => decode_utf8(bytes, partial),
    }
}

fn decode_utf16(body: &[u8], unit: fn([u8; 2]) -> u16, partial: bool) -> Option<String> {
    let (pairs, _) = body.as_chunks::<2>();
    let mut units: Vec<u16> = pairs.iter().map(|pair| unit(*pair)).collect();
    if partial
        && units
            .last()
            .is_some_and(|last| HIGH_SURROGATES.contains(last))
    {
        units.pop();
    }
    String::from_utf16(&units).ok()
}

fn decode_utf8(bytes: Vec<u8>, partial: bool) -> Option<String> {
    let probe = &bytes[..bytes.len().min(BINARY_PROBE_BYTES)];
    if probe.contains(&0) {
        return None;
    }
    match String::from_utf8(bytes) {
        Ok(text) => Some(text),
        Err(error) if partial && error.utf8_error().error_len().is_none() => {
            let valid = error.utf8_error().valid_up_to();
            let mut bytes = error.into_bytes();
            bytes.truncate(valid);
            String::from_utf8(bytes).ok()
        }
        Err(_) => None,
    }
}
