use std::{ops::Deref, str::FromStr};

use crate::error::Error;

#[derive(Clone, Copy, Eq, Hash, PartialEq)]
pub struct UserAgent(pub [u8; 64]);

impl Deref for UserAgent {
    type Target = [u8; 64];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl FromStr for UserAgent {
    type Err = Error;

    fn from_str(string: &str) -> Result<UserAgent, Error> {
        let mut bytes = string.as_bytes().iter();
        let array = [b' '; 64].map(|_| *bytes.next().unwrap_or(&b' '));
        Ok(UserAgent(array))
    }
}
