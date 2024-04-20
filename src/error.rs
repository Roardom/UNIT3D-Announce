use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};

use thiserror::Error;

#[derive(Error, Debug)]
pub enum AnnounceError {
    #[error("Internal tracker error.")]
    InternalTrackerError,
    #[error("Invalid query string parameter.")]
    InvalidQueryStringKey,
    #[error("Invalid query string value.")]
    InvalidQueryStringValue,
    #[error("Invalid 'peer_id'.")]
    InvalidPeerId,
    #[error("Invalid 'info_hash'.")]
    InvalidInfoHash,
    #[error("Invalid 'port' (must be greater than or equal to 0).")]
    InvalidPort,
    #[error("Invalid 'port' (must be greater than or equal to 0).")]
    InvalidUploaded,
    #[error("Invalid 'uploaded' (must be greater than or equal to 0).")]
    InvalidDownloaded,
    #[error("Invalid 'downloaded' (must be greater than or equal to 0).")]
    InvalidLeft,
    #[error("Your client does not support compact announces.")]
    InvalidCompact,
    #[error("Unsupported 'event' type.")]
    UnsupportedEvent,
    #[error("Invalid 'numwant' (must be greater than or equal to 0).")]
    InvalidNumwant,
    #[error("Query parameter 'info_hash' is missing.")]
    MissingInfoHash,
    #[error("Query parameter 'peer_id' is missing.")]
    MissingPeerId,
    #[error("Query parameter 'port' is missing.")]
    MissingPort,
    #[error("Query parameter 'uploaded' is missing.")]
    MissingUploaded,
    #[error("Query parameter 'downloaded' is missing.")]
    MissingDownloaded,
    #[error("Query parameter 'left' is missing.")]
    MissingLeft,
    #[error("Abnormal access blocked.")]
    AbnormalAccess,
    #[error("Invalid user agent.")]
    InvalidUserAgent,
    #[error("The user agent of this client is too long.")]
    UserAgentTooLong,
    #[error("Client is not acceptable. Please check our blacklist.")]
    BlacklistedClient,
    #[error("Browser, crawler or cheater is not allowed.")]
    NotAClient,
    #[error("Invalid passkey.")]
    InvalidPasskey,
    #[error("Passkey does not exist. Please re-download the .torrent file.")]
    PasskeyNotFound,
    #[error("User does not exist. Please re-download the .torrent file.")]
    UserNotFound,
    #[error("Your downloading privileges have been disabled.")]
    DownloadPrivilegesRevoked,
    #[error("Illegal port. Port should be between 6881-64999.")]
    BlacklistedPort,
    #[error("InfoHash not found.")]
    InfoHashNotFound,
    #[error("Torrent not found.")]
    TorrentNotFound,
    #[error("Torrent has been deleted.")]
    TorrentIsDeleted,
    #[error("Torrent is pending moderation.")]
    TorrentIsPendingModeration,
    #[error("Torrent has been rejected.")]
    TorrentIsRejected,
    #[error("Torrent has been postponed.")]
    TorrentIsPostponed,
    #[error("Torrent not approved.")]
    TorrentUnknownModerationStatus,
    #[error("Stopped peer doesn't exist.")]
    StoppedPeerDoesntExist,
    #[error("Group not found.")]
    GroupNotFound,
    #[error("Your account is not enabled. (Current: Banned).")]
    GroupBanned,
    #[error("Your account is not enabled. (Current: Validating).")]
    GroupValidating,
    #[error("Your account is not enabled. (Current: Disabled).")]
    GroupDisabled,
    #[error("You already have 3 peers on this torrent. Ignoring.")]
    PeersPerTorrentPerUserLimit,
}

impl IntoResponse for AnnounceError {
    fn into_response(self) -> Response {
        (
            StatusCode::OK,
            format!(
                "d14:failure reason{}:{}8:intervali5400e12:min intervali5400ee",
                self.to_string().chars().count(),
                self,
            ),
        )
            .into_response()
    }
}

#[derive(Error, Debug)]
pub enum DecodeError {
    #[error("Invalid infohash.")]
    InfoHash,
}
