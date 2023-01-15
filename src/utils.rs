use crate::Error;

/// Decodes a url-encoded string to 20 bytes. Used for decoding the peer_id
/// and infohash from the HTTP GET request query string.
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
            *out_byte = *hex::decode([input[in_pos + 1], input[in_pos + 2]])
                .map_err(|_| Error("Invalid 'info_hash' or 'peer_id' (Invalid URL encoding)."))?
                .first()
                .ok_or(Error(
                    "Invalid 'info_hash' or 'peer_id' (Invalid URL encoding).",
                ))?;
            in_pos += 3;
        } else {
            *out_byte = byte;
            in_pos += 1;
        }
    }

    Ok(output)
}
