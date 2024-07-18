use thiserror::Error;

#[derive(Error, Debug, Clone, Copy, PartialEq)]
pub enum AnnounceWarning {
    #[error("Stopped peer doesn't exist.")]
    StoppedPeerDoesntExist,
    #[error("Rate limit exceeded. Please wait.")]
    RateLimitExceeded,
    #[error("Download slot limit reached")]
    HitDownloadSlotLimit,
    #[error("Connectivity issue detected. Enable port-forwarding to resolve.")]
    ConnectivityIssueDetected,
}
