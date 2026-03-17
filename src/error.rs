/// Errors that can occur during M2PA message processing.
#[derive(Debug, thiserror::Error)]
pub enum M2paError {
    #[error("invalid version: expected 1, got {0}")]
    InvalidVersion(u8),

    #[error("spare field must be 0, got {0}")]
    InvalidSpare(u8),

    #[error("invalid message class: expected 11, got {0}")]
    InvalidMessageClass(u8),

    #[error("invalid message type: expected 1 (User Data) or 2 (Link Status), got {0}")]
    InvalidMessageType(u8),

    #[error("message too short: expected at least {expected} bytes, got {actual}")]
    TooShort { expected: usize, actual: usize },

    #[error("invalid link status value: {0}")]
    InvalidLinkStatus(u32),
}
