use std::{fmt, ops::Deref, str::FromStr};

use sqlx::{database::HasValueRef, Database, Decode};

use crate::error::Error;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct InfoHash(pub [u8; 20]);

impl FromStr for InfoHash {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut bytes = [0u8; 20];
        hex::decode_to_slice(s, &mut bytes as &mut [u8]).map_err(|_| Error("Invalid infohash."))?;

        Ok(InfoHash(bytes))
    }
}

impl From<[u8; 20]> for InfoHash {
    fn from(array: [u8; 20]) -> Self {
        InfoHash(array)
    }
}

impl fmt::Display for InfoHash {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        fmt.write_str(&hex::encode(self.0))
    }
}

impl<'r, DB: Database> Decode<'r, DB> for InfoHash
where
    &'r str: Decode<'r, DB>,
{
    /// Decodes the database's string representation of the 40 character long
    /// infohash in hex into a byte slice
    fn decode(
        value: <DB as HasValueRef<'r>>::ValueRef,
    ) -> Result<InfoHash, Box<dyn std::error::Error + 'static + Send + Sync>> {
        let value = <&str as Decode<DB>>::decode(value)?;

        match InfoHash::from_str(value) {
            Ok(infohash) => Ok(infohash),
            Err(e) => {
                let error: Box<dyn std::error::Error + Send + Sync> = Box::new(e);
                Err(error)
            }
        }
    }
}

impl Deref for InfoHash {
    type Target = [u8; 20];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
