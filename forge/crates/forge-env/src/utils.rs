/// Shared utility functions for forge-env.

/// Encode bytes as standard (unpadded-where-needed, standard alphabet) base64.
///
/// This is a minimal table-driven encoder that avoids adding an external crate
/// dependency.  The output is safe to pass through `echo '...' | base64 -d`
/// inside a bash session.
pub(crate) fn base64_encode(data: &[u8]) -> String {
    // Simple table-driven encoder to avoid adding a dependency.
    const ALPHABET: &[u8; 64] =
        b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

    let mut out = Vec::with_capacity((data.len() + 2) / 3 * 4);
    for chunk in data.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = if chunk.len() > 1 { chunk[1] as u32 } else { 0 };
        let b2 = if chunk.len() > 2 { chunk[2] as u32 } else { 0 };

        let combined = (b0 << 16) | (b1 << 8) | b2;
        out.push(ALPHABET[((combined >> 18) & 0x3F) as usize]);
        out.push(ALPHABET[((combined >> 12) & 0x3F) as usize]);
        out.push(if chunk.len() > 1 {
            ALPHABET[((combined >> 6) & 0x3F) as usize]
        } else {
            b'='
        });
        out.push(if chunk.len() > 2 {
            ALPHABET[(combined & 0x3F) as usize]
        } else {
            b'='
        });
    }

    // SAFETY: base64 alphabet is ASCII-only, so output is always valid UTF-8.
    unsafe { String::from_utf8_unchecked(out) }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base64_encode_hello() {
        // "hello" in base64 is "aGVsbG8="
        assert_eq!(base64_encode(b"hello"), "aGVsbG8=");
    }

    #[test]
    fn base64_encode_empty() {
        assert_eq!(base64_encode(b""), "");
    }

    #[test]
    fn base64_encode_foobar() {
        // "foobar" → "Zm9vYmFy"
        assert_eq!(base64_encode(b"foobar"), "Zm9vYmFy");
    }
}
