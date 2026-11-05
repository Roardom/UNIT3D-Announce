use std::fmt::{Display, Formatter};

/// Announce warnings that don't stop the announce processing but still return
/// an empty peer list.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AnnounceWarning {
    RateLimitExceeded,
    HitDownloadSlotLimit,
    ConnectivityIssueDetected,
    SeedListRateLimitExceeded { window: f64, retry_after: f64 },
    LeechListRateLimitExceeded { window: f64, retry_after: f64 },
}

impl AnnounceWarning {
    /// Returns true if the warning should not be sent to the user
    /// but still used internally to give empty peer lists.
    fn is_silent(&self) -> bool {
        match self {
            // Silenced because it's unlikely qBittorrent will ever fix its
            // duplicate announce bug.
            Self::RateLimitExceeded => true,
            _ => false,
        }
    }
}

fn format_duration(seconds: f64) -> String {
    if !seconds.is_finite() {
        return "later".to_string();
    }

    let seconds = seconds.max(0f64).ceil() as u64;

    match seconds {
        0 => "now".to_string(),
        1..=59 => format!("{seconds}s"),
        60..=3599 => format!("{}m", seconds / 60),
        3600..=86399 => format!("{}h", seconds / 3600),
        86400..=604799 => format!("{}d", seconds / 86400),
        _ => format!("{}w", seconds / 604800),
    }
}

impl Display for AnnounceWarning {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::RateLimitExceeded => write!(f, "Rate limit exceeded. Please wait."),
            Self::HitDownloadSlotLimit => write!(f, "Download slot limit reached"),
            Self::ConnectivityIssueDetected => {
                write!(
                    f,
                    "Connectivity issue detected. Enable port-forwarding to resolve."
                )
            }
            Self::SeedListRateLimitExceeded {
                window,
                retry_after,
            } => write!(
                f,
                "Seed peer list limit hit ({} window). Retry in {}.",
                format_duration(*window),
                format_duration(*retry_after)
            ),
            Self::LeechListRateLimitExceeded {
                window,
                retry_after,
            } => write!(
                f,
                "Leech peer list limit hit ({} window). Retry in {}.",
                format_duration(*window),
                format_duration(*retry_after)
            ),
        }
    }
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

    /// Create the warning message to be returned to the user.
    pub fn into_message(self) -> Option<Vec<u8>> {
        if self.warnings.is_empty() {
            return None;
        }

        let mut message: Vec<u8> = Vec::with_capacity(self.max_byte_length());

        self.warnings.into_iter().for_each(|warning| {
            if !warning.is_silent() {
                message.extend(warning.to_string().as_bytes());
                message.extend(SEPARATOR);
            }
        });

        if message.is_empty() {
            return None;
        }

        // remove the last separator
        message.truncate(message.len() - SEPARATOR.len());

        Some(message)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rate_limit_warning_names_the_window_and_retry() {
        let warning = AnnounceWarning::SeedListRateLimitExceeded {
            window: 900f64,
            retry_after: 600f64,
        };

        assert_eq!(
            warning.to_string(),
            "Seed peer list limit hit (15m window). Retry in 10m."
        );
    }

    #[test]
    fn silent_warning_is_not_serialized() {
        let mut warnings = WarningCollection::new();
        warnings.add(AnnounceWarning::RateLimitExceeded);

        assert!(warnings.into_message().is_none());
    }
}
