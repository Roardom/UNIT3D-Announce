use crate::Error;

/// Decodes a url-encoded string to 20 bytes.
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

/// Encodes one byte into 2 ascii-encoded hex digits.
#[inline(always)]
pub fn hex_encode(char: u8) -> [u8; 2] {
    let char_1 = char >> 4;
    let char_2 = char & 0x0F;
    [
        match char_1 {
            0x0..=0x9 => char_1 + b'0',
            0xA..=0xF => char_1 - 0xA + b'A',
            _ => unreachable!(),
        },
        match char_2 {
            0x0..=0x9 => char_2 + b'0',
            0xA..=0xF => char_2 - 0xA + b'A',
            _ => unreachable!(),
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn urlencoded_all_percents() -> Result<(), Error> {
        let url_encoded = "%00%01%02%03%04%05%06%07%08%09%0A%0B%0C%0D%0E%0F%00%01%02%03";
        let bytes = urlencoded_to_bytes(url_encoded).await?;
        assert_eq!(
            bytes,
            [
                0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0A, 0x0B, 0x0C, 0x0D,
                0x0E, 0x0F, 0x00, 0x01, 0x02, 0x03
            ]
        );
        Ok(())
    }

    #[tokio::test]
    async fn urlencoded_no_percents() -> Result<(), Error> {
        let url_encoded = "33333333333333333333";
        let bytes = urlencoded_to_bytes(url_encoded).await?;
        assert_eq!(
            bytes,
            // ASCII character '3' is 0x33 in hex
            [
                0x33, 0x33, 0x33, 0x33, 0x33, 0x33, 0x33, 0x33, 0x33, 0x33, 0x33, 0x33, 0x33, 0x33,
                0x33, 0x33, 0x33, 0x33, 0x33, 0x33
            ]
        );
        Ok(())
    }

    #[tokio::test]
    async fn urlencoded_mixed_percents() -> Result<(), Error> {
        let url_encoded = "%00%01%02%03%04%05%06%07%08%09%0A%0B%0C%0D%0E%0F3333";
        let bytes = urlencoded_to_bytes(url_encoded).await?;
        assert_eq!(
            bytes,
            [
                0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0A, 0x0B, 0x0C, 0x0D,
                0x0E, 0x0F, 0x33, 0x33, 0x33, 0x33
            ]
        );
        Ok(())
    }

    #[tokio::test]
    async fn urlencoded_incorrect_length() {
        let url_encoded = "%00";
        let bytes = urlencoded_to_bytes(url_encoded).await;
        assert!(bytes.is_err());
    }

    #[tokio::test]
    async fn urlencoded_too_many_percents() {
        let url_encoded = "%0%%01%02%03%04%05%06%07%08%09%0A%0B%0C%0D%0E%0F%00%01%02%03";
        let bytes = urlencoded_to_bytes(url_encoded).await;
        assert!(bytes.is_err());
    }

    #[test]
    fn hex_decode_lowercase() -> Result<(), Error> {
        let lowercase_ascii_chars: [u8; 2] = [b'7', b'c'];
        let hex = hex_decode(lowercase_ascii_chars)?;
        assert_eq!(hex, 0x7C);
        Ok(())
    }

    #[test]
    fn hex_decode_uppercase() -> Result<(), Error> {
        let uppercase_ascii_chars: [u8; 2] = [b'7', b'C'];
        let hex = hex_decode(uppercase_ascii_chars)?;
        assert_eq!(hex, 0x7C);
        Ok(())
    }

    #[test]
    fn hex_decode_invalid() -> Result<(), Error> {
        let invalid_ascii_chars: [u8; 2] = [b'z', b'z'];
        let hex = hex_decode(invalid_ascii_chars);
        assert!(hex.is_err());
        Ok(())
    }

    #[test]
    fn hex_encode_letters() {
        let hex: u8 = 0xCC;
        let chars = hex_encode(hex);
        assert_eq!(chars, [b'C', b'C']);
    }

    #[test]
    fn hex_encode_numbers() {
        let hex: u8 = 0x77;
        let chars = hex_encode(hex);
        assert_eq!(chars, [b'7', b'7']);
    }

    #[test]
    fn hex_encode_mixed() {
        let hex: u8 = 0x7C;
        let chars = hex_encode(hex);
        assert_eq!(chars, [b'7', b'C']);
    }
}
