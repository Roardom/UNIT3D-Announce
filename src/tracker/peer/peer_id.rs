use std::ops::Deref;

use sqlx::{database::HasValueRef, Database, Decode};

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
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

impl From<&[u8]> for PeerId {
    fn from(slice: &[u8]) -> Self {
        let peer_id: [u8; 20] = slice.try_into().expect("Invalid peer id.");
        PeerId(peer_id)
    }
}

impl<'r, DB: Database> Decode<'r, DB> for PeerId
where
    &'r [u8]: Decode<'r, DB>,
{
    /// Decodes the database's 2-byte binary peer_id
    fn decode(
        value: <DB as HasValueRef<'r>>::ValueRef,
    ) -> Result<PeerId, Box<dyn std::error::Error + 'static + Send + Sync>> {
        let value = <&[u8] as Decode<DB>>::decode(value)?;

        match value.try_into() {
            Ok(peer_id) => Ok(peer_id),
            Err(e) => {
                let error: Box<dyn std::error::Error + Send + Sync> = Box::new(e);
                Err(error)
            }
        }
    }
}
