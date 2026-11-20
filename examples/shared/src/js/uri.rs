//! What the web encodes and decodes for an address or a file: base64 as `btoa` and `atob` make and
//! read it, a query string as `URLSearchParams` reads it, and `encodeURIComponent` and
//! `decodeURIComponent`.

const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

/// Bytes as `btoa` writes them, padded.
pub fn base64_encode(bytes: &[u8]) -> String {
    let mut out = String::new();
    for chunk in bytes.chunks(3) {
        let n = (u32::from(chunk[0]) << 16)
            | (u32::from(*chunk.get(1).unwrap_or(&0)) << 8)
            | u32::from(*chunk.get(2).unwrap_or(&0));
        for at in 0..4 {
            if at <= chunk.len() {
                out.push(ALPHABET[((n >> (18 - 6 * at)) & 63) as usize] as char);
            } else {
                out.push('=');
            }
        }
    }
    out
}

/// Base64 read as `atob` reads it: spaces left out, the padding optional; none for anything it
/// refuses.
pub fn base64_decode(text: &str) -> Option<Vec<u8>> {
    let mut clean: Vec<u8> = text
        .bytes()
        .filter(|byte| !matches!(byte, b'\t' | b'\n' | b'\x0c' | b'\r' | b' '))
        .collect();
    if clean.len().is_multiple_of(4) {
        for _ in 0..2 {
            if clean.last() == Some(&b'=') {
                clean.pop();
            }
        }
    }
    if clean.len() % 4 == 1 {
        return None;
    }
    let mut out = Vec::new();
    let mut bits = 0u32;
    let mut count = 0;
    for byte in clean {
        let value = ALPHABET.iter().position(|one| *one == byte)? as u32;
        bits = (bits << 6) | value;
        count += 6;
        if count >= 8 {
            count -= 8;
            out.push((bits >> count) as u8);
            bits &= (1 << count) - 1;
        }
    }
    Some(out)
}

fn decode_form(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut out = Vec::new();
    let mut at = 0;
    while at < bytes.len() {
        match bytes[at] {
            b'+' => out.push(b' '),
            b'%' if at + 2 < bytes.len() && bytes[at + 1].is_ascii_hexdigit() && bytes[at + 2].is_ascii_hexdigit() => {
                out.push(u8::from_str_radix(&text[at + 1..at + 3], 16).unwrap_or(b'%'));
                at += 3;
                continue;
            }
            byte => out.push(byte),
        }
        at += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// A query string's pairs as `new URLSearchParams(text).entries()` gives them.
pub fn search_params(text: &str) -> Vec<(String, String)> {
    let text = text.strip_prefix('?').unwrap_or(text);
    text.split('&')
        .filter(|piece| !piece.is_empty())
        .map(|piece| match piece.split_once('=') {
            Some((key, value)) => (decode_form(key), decode_form(value)),
            None => (decode_form(piece), String::new()),
        })
        .collect()
}

/// A text as `encodeURIComponent` writes it: every byte of its UTF-8 but the unreserved ones as
/// `%XX`.
pub fn encode_uri_component(text: &str) -> String {
    let mut out = String::new();
    for byte in text.bytes() {
        if byte.is_ascii_alphanumeric() || b"-_.!~*'()".contains(&byte) {
            out.push(byte as char);
        } else {
            out.push_str(&format!("%{byte:02X}"));
        }
    }
    out
}

/// Words read back from an address, as `decodeURIComponent` reads them.
pub fn decode_uri_component(words: &str) -> String {
    let bytes = words.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        let hex = |b: u8| (b as char).to_digit(16);
        if bytes[i] == b'%'
            && i + 2 < bytes.len()
            && let (Some(high), Some(low)) = (hex(bytes[i + 1]), hex(bytes[i + 2]))
        {
            out.push((high * 16 + low) as u8);
            i += 3;
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}
