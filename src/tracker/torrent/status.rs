use std::fmt::Display;

use diesel::{
    backend::Backend,
    deserialize::{FromSql, FromSqlRow},
    sql_types::SmallInt,
};
use serde::Serialize;
use serde_repr::Deserialize_repr;

/// Torrent moderation status
#[derive(Clone, Copy, Debug, Default, Deserialize_repr, Eq, PartialEq, Serialize, FromSqlRow)]
#[repr(i16)]
pub enum Status {
    /// A torrent with pending status is currently in moderation queue
    /// and have not yet been moderated. Pending torrents are only visible
    /// to moderators and the uploader.
    Pending,
    /// A torrent with approved status has passed the moderation queue
    /// and is available to download on the site for all users.
    Approved,
    /// A torrent with a rejected status is currently in moderation queue
    /// after having already been moderated. A moderator will mark a torrent
    /// as rejected if, after editing, it's not possible to meet site rules.
    /// Rejected torrents are only visible to moderators and the uploader.
    Rejected,
    /// A torrent with postponed status is currently in moderation queue
    /// after having already been moderated. A moderator will mark a torrent
    /// as postponed if it doesn't currently meet site rules, but has
    /// the possibility of meeting site rules after editing. Postponed
    /// torrents are only visible to moderators and the uploader.
    Postponed,
    /// A torrent with an unknown status shouldn't happen, but it has the
    /// possibility of happening until the unit3d database uses enums for
    /// the moderation status instead of a smallint.
    #[default]
    Unknown,
}

impl Status {
    fn from_i16(status: i16) -> Status {
        match status {
            0 => Self::Pending,
            1 => Self::Approved,
            2 => Self::Rejected,
            3 => Self::Postponed,
            _ => Self::Unknown,
        }
    }
}

impl<DB> FromSql<SmallInt, DB> for Status
where
    DB: Backend,
    i16: FromSql<SmallInt, DB>,
{
    fn from_sql(bytes: <DB as Backend>::RawValue<'_>) -> diesel::deserialize::Result<Self> {
        Ok(Status::from_i16(i16::from_sql(bytes)?))
    }
}

impl Display for Status {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pending => f.write_str("Pending"),
            Self::Approved => f.write_str("Approved"),
            Self::Rejected => f.write_str("Rejected"),
            Self::Postponed => f.write_str("Postponed"),
            Self::Unknown => f.write_str("Unknown"),
        }
    }
}
