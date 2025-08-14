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

const SEPARATOR: &[u8] = b"; ";

pub struct WarningCollection {
    warnings: Vec<AnnounceWarning>,
}

impl WarningCollection {
    /// Initializes a new warning collection.
    #[inline(always)]
    pub fn new() -> Self {
        Self {
            warnings: Vec::new(),
        }
    }

    /// Calculates the max byte length of the combined warning message.
    #[inline(always)]
    pub const fn max_byte_length(&self) -> usize {
        // Individual warnings should be limited to less than 64 characters
        // (the limit of some clients)
        const MAX_WARNING_LEN: usize = 64 + SEPARATOR.len();

        self.warnings.len() * MAX_WARNING_LEN
    }

    /// Add a new warning to the collection.
    #[inline(always)]
    pub fn add(&mut self, warning: AnnounceWarning) {
        self.warnings.push(warning);
    }

    /// Returns true if there are no warnings.
    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        self.warnings.is_empty()
    }

    /// Returns true if there exists a warning which requires returning early.
    pub fn should_early_return(&self) -> bool {
        self.warnings
            .contains(&AnnounceWarning::StoppedPeerDoesntExist)
    }

    /// Create the warning message to be returned to the user.
    pub fn into_message(self) -> Option<Vec<u8>> {
        if self.warnings.is_empty() {
            return None;
        }

        let mut message: Vec<u8> = Vec::with_capacity(self.max_byte_length());

        self.warnings.into_iter().for_each(|warning| {
            message.extend(warning.to_string().as_bytes());
            message.extend(SEPARATOR);
        });

        // remove the last separator
        message.truncate(message.len() - SEPARATOR.len());

        Some(message)
    }
}
