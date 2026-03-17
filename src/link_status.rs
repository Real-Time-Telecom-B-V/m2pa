use std::fmt;

use crate::error::M2paError;

/// Link Status states as defined in RFC 4165 Section 3.3.
///
/// ```ignore
///   Value       Description
///   -----       -----------
///     1         Alignment
///     2         Proving Normal
///     3         Proving Emergency
///     4         Ready
///     5         Processor Outage
///     6         Processor Recovered
///     7         Busy
///     8         Busy Ended
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum LinkState {
    Alignment = 1,
    ProvingNormal = 2,
    ProvingEmergency = 3,
    Ready = 4,
    ProcessorOutage = 5,
    ProcessorRecovered = 6,
    Busy = 7,
    BusyEnded = 8,
}

impl LinkState {
    pub fn from_u32(value: u32) -> Result<Self, M2paError> {
        match value {
            1 => Ok(Self::Alignment),
            2 => Ok(Self::ProvingNormal),
            3 => Ok(Self::ProvingEmergency),
            4 => Ok(Self::Ready),
            5 => Ok(Self::ProcessorOutage),
            6 => Ok(Self::ProcessorRecovered),
            7 => Ok(Self::Busy),
            8 => Ok(Self::BusyEnded),
            other => Err(M2paError::InvalidLinkStatus(other)),
        }
    }
}

impl fmt::Display for LinkState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Alignment => write!(f, "Alignment"),
            Self::ProvingNormal => write!(f, "Proving Normal"),
            Self::ProvingEmergency => write!(f, "Proving Emergency"),
            Self::Ready => write!(f, "Ready"),
            Self::ProcessorOutage => write!(f, "Processor Outage"),
            Self::ProcessorRecovered => write!(f, "Processor Recovered"),
            Self::Busy => write!(f, "Busy"),
            Self::BusyEnded => write!(f, "Busy Ended"),
        }
    }
}

/// Link Status message body (follows M2PA header).
///
/// RFC 4165 Section 3.3:
/// ```ignore
/// 0                   1                   2                   3
/// 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
/// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
/// |                         State                                 |
/// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
/// |                       Filler                                  |
/// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
/// ```
///
/// Link Status messages are sent on SCTP stream 0.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LinkStatusMessage {
    pub state: LinkState,
}

impl LinkStatusMessage {
    /// SCTP stream used for Link Status messages.
    pub const SCTP_STREAM: u16 = 0;

    pub fn new(state: LinkState) -> Self {
        Self { state }
    }

    /// Decode from the 4-byte state field (after M2PA header).
    pub fn decode(bytes: &[u8]) -> Result<Self, M2paError> {
        if bytes.len() < 4 {
            return Err(M2paError::TooShort {
                expected: 4,
                actual: bytes.len(),
            });
        }
        let value = u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
        let state = LinkState::from_u32(value)?;
        Ok(Self { state })
    }

    /// Encode the state field to 4 bytes (big-endian).
    pub fn encode(&self) -> [u8; 4] {
        (self.state as u32).to_be_bytes()
    }
}

impl fmt::Display for LinkStatusMessage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Link Status [state={}]", self.state)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn link_state_round_trip() {
        for val in 1..=8u32 {
            let state = LinkState::from_u32(val).unwrap();
            let msg = LinkStatusMessage::new(state);
            let encoded = msg.encode();
            let decoded = LinkStatusMessage::decode(&encoded).unwrap();
            assert_eq!(decoded, msg);
        }
    }

    #[test]
    fn link_state_invalid() {
        assert!(LinkState::from_u32(0).is_err());
        assert!(LinkState::from_u32(9).is_err());
        assert!(LinkState::from_u32(255).is_err());
    }

    #[test]
    fn decode_proving_emergency() {
        let bytes = [0x00, 0x00, 0x00, 0x03];
        let msg = LinkStatusMessage::decode(&bytes).unwrap();
        assert_eq!(msg.state, LinkState::ProvingEmergency);
    }

    #[test]
    fn decode_ready() {
        let bytes = [0x00, 0x00, 0x00, 0x04];
        let msg = LinkStatusMessage::decode(&bytes).unwrap();
        assert_eq!(msg.state, LinkState::Ready);
    }

    #[test]
    fn decode_too_short() {
        let bytes = [0x00, 0x00];
        assert!(LinkStatusMessage::decode(&bytes).is_err());
    }

    #[test]
    fn link_state_display() {
        assert_eq!(format!("{}", LinkState::ProvingEmergency), "Proving Emergency");
        assert_eq!(format!("{}", LinkState::Ready), "Ready");
    }
}
