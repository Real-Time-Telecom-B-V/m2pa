use std::fmt;

use crate::error::M2paError;

/// User Data message body (follows M2PA header).
///
/// RFC 4165 Section 3.2:
/// ```ignore
/// 0                   1                   2                   3
/// 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
/// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
/// |       Priority                |            User Data          |
/// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
/// |                          User Data                            |
/// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
/// ```
///
/// User Data messages are sent on SCTP stream 1.
/// The User Data field carries an MTP3 MSU (Message Signal Unit).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserDataMessage {
    /// Priority of the message (0-3).
    pub priority: u8,
    /// MTP3 MSU payload.
    pub msu: Vec<u8>,
}

impl UserDataMessage {
    /// SCTP stream used for User Data messages.
    pub const SCTP_STREAM: u16 = 1;

    pub fn new(priority: u8, msu: Vec<u8>) -> Self {
        Self {
            priority: priority & 0x03, // Only 2 bits used
            msu,
        }
    }

    /// Decode from bytes after M2PA header.
    ///
    /// Layout: 1 byte priority (upper byte of 2-byte field) + N bytes MSU.
    /// The priority is in the first byte; the second byte of the priority field
    /// is spare. The MSU follows.
    pub fn decode(bytes: &[u8]) -> Result<Self, M2paError> {
        // Need at least the 1-byte priority field
        if bytes.is_empty() {
            return Err(M2paError::TooShort {
                expected: 1,
                actual: 0,
            });
        }
        let priority = bytes[0] & 0x03;
        let msu = bytes[1..].to_vec();
        Ok(Self { priority, msu })
    }

    /// Encode to bytes: 1 byte priority + MSU.
    pub fn encode(&self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(1 + self.msu.len());
        buf.push(self.priority & 0x03);
        buf.extend_from_slice(&self.msu);
        buf
    }
}

impl fmt::Display for UserDataMessage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "User Data [priority={}, msu_len={}]",
            self.priority,
            self.msu.len()
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip() {
        let msu = vec![0x83, 0x01, 0x02, 0x03, 0x04, 0x05];
        let msg = UserDataMessage::new(2, msu.clone());
        let encoded = msg.encode();
        let decoded = UserDataMessage::decode(&encoded).unwrap();
        assert_eq!(decoded.priority, 2);
        assert_eq!(decoded.msu, msu);
    }

    #[test]
    fn priority_masked_to_2_bits() {
        let msg = UserDataMessage::new(0xFF, vec![0x01]);
        assert_eq!(msg.priority, 3); // 0xFF & 0x03 = 3
    }

    #[test]
    fn decode_empty_fails() {
        assert!(UserDataMessage::decode(&[]).is_err());
    }

    #[test]
    fn display() {
        let msg = UserDataMessage::new(1, vec![0; 10]);
        assert_eq!(format!("{msg}"), "User Data [priority=1, msu_len=10]");
    }
}
