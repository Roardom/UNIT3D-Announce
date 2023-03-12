use crate::Error;

/// Decodes a url-encoded string to 20 bytes.
///
/// # Example
///
/// ```
/// let url_encoded = "%00%01%02%03%04%05%06%07%08%09%0A%0B%0C%0D%0E%0F%00%01%02%03";
/// assert_eq!(url_encoded_to_bytes(url_encoded), 0x000102030405060708090A0B0C0D0E0F00010203);
/// let url_encoded = "33333333333333333333";
/// assert_eq!(url_encoded_to_bytes(url_encoded), 0x3333333333333333333333333333333333333333);
/// let url_encoded = "%00%01%02%03%04%05%06%07%08%09%0A%0B%0C%0D%0E%0F3333";
/// assert_eq!(url_encoded_to_bytes(url_encoded), 0x000102030405060708090A0B0C0D0E0F33333333);
/// let url_encoded = "%00";
/// assert_eq!(url_encoded_to_bytes(url_encoded)is_err());
/// ```
///
/// Used for decoding the peer_id and infohash from the HTTP GET request query string.
#[inline(always)]
pub async fn urlencoded_to_bytes(input: &str) -> Result<[u8; 20], Error> {
    let mut output: [u8; 20] = [0; 20];
    let input = input.as_bytes();
    let percent_sign_count = memchr::memchr_iter(b'%', input).count();

    if input.len() != 20 + 2 * percent_sign_count {
        return Err(Error(
            "Invalid 'info_hash' or 'peer_id' (must be 20 bytes long).",
        ));
    }

    let mut in_pos = 0;

    for out_byte in &mut output.iter_mut() {
        let byte = input[in_pos];

        if byte == b'%' {
            *out_byte = hex_decode([input[in_pos + 1], input[in_pos + 2]])?;
            in_pos += 3;
        } else {
            *out_byte = byte;
            in_pos += 1;
        }
    }

    Ok(output)
}

/// Decodes two ascii-encoded hex digits into one byte.
///
/// # Examples
///
/// ```
/// let lowercase_ascii_char: u8 = b"7c";
/// assert_eq!(hex_decode(lowercase_ascii_char), 0x7C);
/// let uppercase_ascii_char: u8 = b"7C";
/// assert_eq!(hex_decode(uppercase_ascii_char), 0x7C);
/// let incompatible_ascii_char: u8 = b"zz";
/// assert_eq!(hex_decode(incompatible_ascii_char).is_err());
/// ```
#[inline(always)]
pub fn hex_decode(chars: [u8; 2]) -> Result<u8, Error> {
    Ok(match chars[0] {
        b'0'..=b'9' => chars[0] - b'0' << 4,
        b'a'..=b'f' => chars[0] - b'a' + 0xA << 4,
        b'A'..=b'F' => chars[0] - b'A' + 0xA << 4,
        _ => Err(Error("Invalid URL encoding."))?,
    } + match chars[1] {
        b'0'..=b'9' => chars[1] - b'0',
        b'a'..=b'f' => chars[1] - b'a' + 0xA,
        b'A'..=b'F' => chars[1] - b'A' + 0xA,
        _ => Err(Error("Invalid URL encoding."))?,
    })
}

/// Encodes one byte into 2 hex digits.
///
/// # Example
///
/// ```
/// let byte: u8 = 0x7C;
/// assert_eq!(hex_encode(byte), b"7C");
/// ```
#[inline(always)]
pub fn hex_encode(char: u8) -> [u8; 2] {
    [
        match char >> 4 {
            0x0..=0x9 => char + b'0',
            0xA..=0xF => char + b'A',
            _ => unreachable!(),
        },
        match char & 0x0F {
            0x0..=0x9 => char + b'0',
            0xA..=0xF => char + b'A',
            _ => unreachable!(),
        },
    ]
}
