use std::{fmt, ops::Deref, str::FromStr};

use serde::Deserialize;
use sqlx::{Database, Decode};

use crate::utils::{hex_decode, hex_encode};

use anyhow::{Context, Result, bail};

#[derive(Clone, Copy, Deserialize, Debug, Eq, Hash, PartialEq, PartialOrd, Ord)]
pub struct InfoHash(pub [u8; 20]);

impl FromStr for InfoHash {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let bytes = s.as_bytes();
        let mut out = [0u8; 20];

        if bytes.len() != 40 {
            bail!("`{s}` is not a valid infohash.");
        }

        for pos in 0..20 {
            out[pos] = hex_decode([bytes[pos * 2], bytes[pos * 2 + 1]])
                .context("`{s}` is not a valid infohash")?;
        }

        Ok(InfoHash(out))
    }
}

impl From<[u8; 20]> for InfoHash {
    fn from(array: [u8; 20]) -> Self {
        InfoHash(array)
    }
}

impl fmt::Display for InfoHash {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        let mut bytes: Vec<u8> = vec![];

        for pos in 0..20 {
            bytes.extend(hex_encode(self.0[pos]));
        }

        fmt.write_str(&String::from_utf8_lossy(&bytes))
    }
}

impl<'r, DB: Database> Decode<'r, DB> for InfoHash
where
    &'r [u8]: Decode<'r, DB>,
{
    /// Decodes the database's string representation of the 40 character long
    /// infohash in hex into a byte slice
    fn decode(
        value: <DB as Database>::ValueRef<'r>,
    ) -> Result<InfoHash, Box<dyn std::error::Error + 'static + Send + Sync>> {
        let value = <&[u8] as Decode<DB>>::decode(value)?;

        if value.len() != 20 {
            let error: Box<dyn std::error::Error + Send + Sync> =
                Box::new(crate::error::DecodeError::InfoHash);

            return Err(error);
        }

        Ok(InfoHash(<[u8; 20]>::try_from(&value[0..20])?))
    }
}

impl Deref for InfoHash {
    type Target = [u8; 20];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
