use std::{
    fmt::{Debug, Display},
    ops::Deref,
};

use diesel::{
    backend::Backend,
    deserialize::{self, FromSql, FromSqlRow},
    sql_types::Binary,
};
use serde::{Serialize, Serializer};

use crate::utils::hex_encode;

#[derive(Clone, Copy, Eq, Hash, PartialEq, FromSqlRow)]
#[diesel(sql_type = Binary)]
pub struct PeerId(pub [u8; 20]);

impl Deref for PeerId {
    type Target = [u8; 20];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<[u8; 20]> for PeerId {
    fn from(array: [u8; 20]) -> Self {
        PeerId(array)
    }
}
impl TryFrom<Vec<u8>> for PeerId {
    type Error = &'static str;

    fn try_from(value: Vec<u8>) -> std::prelude::v1::Result<Self, Self::Error> {
        if value.len() != 20 {
            return Err("Peer id must be 20 bytes.");
        }

        Ok(Self(
            <[u8; 20]>::try_from(value).map_err(|_| "Invalid peer id.")?,
        ))
    }
}

impl From<&[u8]> for PeerId {
    fn from(slice: &[u8]) -> Self {
        let peer_id: [u8; 20] = slice.try_into().expect("Invalid peer id.");
        PeerId(peer_id)
    }
}

impl<DB> FromSql<Binary, DB> for PeerId
where
    DB: Backend,
    Vec<u8>: FromSql<Binary, DB>,
{
    /// Decodes the database's 20-byte binary peer_id
    fn from_sql(bytes: DB::RawValue<'_>) -> diesel::deserialize::Result<Self> {
        Ok(Self::try_from(<Vec<u8>>::from_sql(bytes)?)?)
    }
}

impl Display for PeerId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut hex = [0u8; 40];

        for i in 0..self.0.len() {
            [hex[2 * i], hex[2 * i + 1]] = hex_encode(self.0[i]);
        }

        f.write_str(&String::from_utf8_lossy(&hex))
    }
}

impl Debug for PeerId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.to_string())
    }
}

impl Serialize for PeerId {
    fn serialize<S>(&self, serializer: S) -> std::prelude::v1::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}
