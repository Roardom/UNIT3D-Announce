use std::{fmt, ops::Deref, str::FromStr};

use diesel::{
    backend::Backend,
    deserialize::{FromSql, FromSqlRow},
    sql_types::Binary,
};
use serde::Deserialize;

use crate::utils::{hex_decode, hex_encode};

use anyhow::{bail, Context, Result};

#[derive(Clone, Copy, Deserialize, Debug, Eq, Hash, PartialEq, FromSqlRow)]
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

impl TryFrom<Vec<u8>> for InfoHash {
    type Error = &'static str;

    fn try_from(value: Vec<u8>) -> std::prelude::v1::Result<Self, Self::Error> {
        if value.len() != 20 {
            return Err("Info Hash must be 20 bytes.");
        }

        Ok(Self(
            <[u8; 20]>::try_from(value).map_err(|_| "Invalid info hash.")?,
        ))
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

impl<DB> FromSql<Binary, DB> for InfoHash
where
    DB: Backend,
    Vec<u8>: FromSql<Binary, DB>,
{
    fn from_sql(bytes: DB::RawValue<'_>) -> diesel::deserialize::Result<Self> {
        Ok(Self::try_from(<Vec<u8>>::from_sql(bytes)?)?)
    }
}

impl Deref for InfoHash {
    type Target = [u8; 20];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
