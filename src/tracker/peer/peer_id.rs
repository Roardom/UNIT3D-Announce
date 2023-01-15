use std::ops::Deref;

#[derive(Clone, Copy, Eq, Hash, PartialEq)]
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
