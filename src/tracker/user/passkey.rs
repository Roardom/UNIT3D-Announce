use std::{
    fmt::{Debug, Display},
    str::FromStr,
};

use diesel::{
    backend::Backend,
    deserialize::{FromSql, FromSqlRow},
    sql_types::VarChar,
};
use serde::{Deserialize, Serialize, Serializer};

use anyhow::bail;

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, FromSqlRow)]
pub struct Passkey(pub [u8; 32]);

impl FromStr for Passkey {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut bytes = s.bytes();

        if bytes.len() != 32 {
            bail!("Invalid passkey length.");
        }

        let array = [(); 32].map(|_| bytes.next().unwrap());

        Ok(Passkey(array))
    }
}

impl TryFrom<String> for Passkey {
    type Error = &'static str;

    fn try_from(value: String) -> std::prelude::v1::Result<Self, Self::Error> {
        if value.len() != 32 {
            return Err("Passkey must be 32 bytes.");
        }

        Ok(Self(
            <[u8; 32]>::try_from(value.as_bytes()).map_err(|_| "Invalid passkey.")?,
        ))
    }
}

impl<DB> FromSql<VarChar, DB> for Passkey
where
    DB: Backend,
    String: FromSql<VarChar, DB>,
{
    fn from_sql(bytes: DB::RawValue<'_>) -> diesel::deserialize::Result<Self> {
        Ok(Self::try_from(String::from_sql(bytes)?)?)
    }
}

impl Display for Passkey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&String::from_utf8_lossy(&self.0))
    }
}

impl Serialize for Passkey {
    fn serialize<S>(&self, serializer: S) -> std::prelude::v1::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}
