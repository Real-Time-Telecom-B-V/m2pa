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

    /// An acknowledgement-only message: no Data field at all (RFC 4165 2.3.1,
    /// 4.2.1). It carries a BSN and is not itself acknowledged.
    pub fn empty() -> Self {
        Self {
            priority: 0,
            msu: Vec::new(),
        }
    }

    /// Whether this is an acknowledgement-only message (no MSU).
    pub fn is_empty(&self) -> bool {
        self.msu.is_empty()
    }

    /// Decode the Data field (the octets after the M2PA header).
    ///
    /// The first octet is `|PRI| spare |` with the priority in its two most
    /// significant bits (RFC 4165 2.3.1); the MSU (SIO + SIF) follows. An absent
    /// Data field is an acknowledgement-only message.
    pub fn decode(bytes: &[u8]) -> Result<Self, M2paError> {
        match bytes.split_first() {
            None => Ok(Self::empty()),
            Some((first, msu)) => Ok(Self {
                priority: first >> 6,
                msu: msu.to_vec(),
            }),
        }
    }

    /// Encode the Data field: the priority octet, then the MSU. A message with
    /// no MSU encodes to nothing, the acknowledgement-only form.
    pub fn encode(&self) -> Vec<u8> {
        if self.msu.is_empty() {
            return Vec::new();
        }
        let mut buf = Vec::with_capacity(1 + self.msu.len());
        buf.push((self.priority & 0x03) << 6);
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
    fn an_absent_data_field_is_an_acknowledgement() {
        // RFC 4165 2.3.1: the Data field "need not be included in an
        // acknowledgement-only message".
        let ack = UserDataMessage::decode(&[]).unwrap();
        assert!(ack.is_empty());
        assert!(UserDataMessage::empty().encode().is_empty());
    }

    #[test]
    fn display() {
        let msg = UserDataMessage::new(1, vec![0; 10]);
        assert_eq!(format!("{msg}"), "User Data [priority=1, msu_len=10]");
    }
}
