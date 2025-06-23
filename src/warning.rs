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

impl AnnounceWarning {
    /// Returns true if the warning should be not be sent to the user
    /// but still used internally to give empty peer lists.
    pub fn is_silent(&self) -> bool {
        match self {
            // Silenced because it's unlikely qBittorrent will ever fix its
            // duplicate announce bug.
            Self::RateLimitExceeded => true,
            _ => false,
        }
    }
}

pub struct WarningCollection {
    warnings: Vec<AnnounceWarning>,
}

impl WarningCollection {
    /// Initializes a new warning collection.
    pub fn new() -> Self {
        Self {
            warnings: Vec::new(),
        }
    }

    /// Calculates the max byte length of the combined warning message.
    pub fn max_byte_length(&self) -> usize {
        // Individual warnings should be limited to less than 64 characters
        // (the limit of some clients)
        const MAX_WARNING_LEN: usize = 64;
        const SEPARATOR_LEN: usize = 2;

        self.warnings.len() * (MAX_WARNING_LEN + SEPARATOR_LEN)
    }

    /// Add a new warning to the collection.
    pub fn add(&mut self, warning: AnnounceWarning) {
        self.warnings.push(warning);
    }

    /// Returns true if there are no warnings.
    pub fn is_empty(&self) -> bool {
        self.warnings.is_empty()
    }

    /// Returns true if there exists a warning which requires returning early.
    pub fn should_early_return(&self) -> bool {
        self.warnings
            .contains(&AnnounceWarning::StoppedPeerDoesntExist)
    }

    /// Create the warning message to be returned to the user.
    pub fn message(&self) -> Option<Vec<u8>> {
        if self.warnings.is_empty() {
            return None;
        }

        let mut message: Vec<u8> = Vec::with_capacity(self.max_byte_length());

        // Join the individual warnings by `; ` between each one

        for i in 0..(self.warnings.len() - 1) {
            let warning = self.warnings.get(i).expect("in range");

            if !warning.is_silent() {
                message.extend(warning.to_string().as_bytes());
                message.extend(b"; ");
            }
        }

        let warning = self.warnings.last().expect("not empty");

        if !warning.is_silent() {
            message.extend(warning.to_string().as_bytes());
        }

        if message.is_empty() {
            return None;
        }

        Some(message)
    }
}
