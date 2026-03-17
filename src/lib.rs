//! M2PA (MTP2 Peer-to-Peer Adaptation Layer) implementation per RFC 4165.
//!
//! M2PA is used to transport MTP3 messages over SCTP associations,
//! providing MTP2-equivalent functionality over IP networks.
//!
//! # Protocol Structure
//!
//! Every M2PA message consists of:
//! 1. Common Message Header (8 bytes) - version, class, type, length
//! 2. M2PA Header (8 bytes) - BSN and FSN sequence numbers
//! 3. Message body - either User Data (MTP3 MSU) or Link Status
//!
//! # SCTP Stream Usage
//!
//! - Stream 0: Link Status messages
//! - Stream 1: User Data messages (MTP3 MSUs)

pub mod error;
pub mod link_status;
pub mod state_machine;
pub mod user_data;

use std::fmt;

use modular_bitfield_msb::{
    bitfield,
    specifiers::{B8, B24, B32},
};

pub use error::M2paError;
pub use link_status::{LinkState, LinkStatusMessage};
pub use state_machine::{M2paState, M2paStateMachine};
pub use user_data::UserDataMessage;

/// Message type: User Data (carries MTP3 MSU).
pub const MESSAGE_TYPE_USER_DATA: u8 = 1;
/// Message type: Link Status.
pub const MESSAGE_TYPE_LINK_STATUS: u8 = 2;
/// Message class for M2PA.
pub const MESSAGE_CLASS_M2PA: u8 = 11;
/// M2PA protocol version.
pub const VERSION: u8 = 1;
/// SCTP Payload Protocol Identifier for M2PA.
pub const SCTP_PPID: u32 = 5;

/// Common Message Header for all M2PA messages (8 bytes).
///
/// ```ignore
/// 0                   1                   2                   3
/// 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
/// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
/// |    Version    |     Spare     | Message Class | Message Type  |
/// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
/// |                        Message Length                         |
/// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
/// ```
///
/// See <https://www.rfc-editor.org/rfc/rfc4165.html#section-2.1>
#[derive(Clone, Copy)]
#[bitfield]
pub struct CommonMessageHeader {
    pub version: B8,
    #[skip(setters)]
    pub spare: B8,
    pub message_class: B8,
    pub message_type: B8,
    pub message_length: B32,
}

impl Default for CommonMessageHeader {
    fn default() -> Self {
        Self::new()
    }
}

impl CommonMessageHeader {
    /// Validate a header against RFC 4165 constraints.
    pub fn validate(header: CommonMessageHeader) -> Result<CommonMessageHeader, M2paError> {
        if header.version() != 1 {
            return Err(M2paError::InvalidVersion(header.version()));
        }
        if header.spare() != 0 {
            return Err(M2paError::InvalidSpare(header.spare()));
        }
        if header.message_class() != MESSAGE_CLASS_M2PA {
            return Err(M2paError::InvalidMessageClass(header.message_class()));
        }
        if header.message_type() != MESSAGE_TYPE_USER_DATA
            && header.message_type() != MESSAGE_TYPE_LINK_STATUS
        {
            return Err(M2paError::InvalidMessageType(header.message_type()));
        }
        Ok(header)
    }

    /// Decode and validate from an 8-byte array.
    pub fn decode(bytes: [u8; 8]) -> Result<CommonMessageHeader, M2paError> {
        CommonMessageHeader::validate(CommonMessageHeader::from_bytes(bytes))
    }

    /// Validate and encode to an 8-byte array.
    pub fn encode(self) -> Result<[u8; 8], M2paError> {
        CommonMessageHeader::validate(self)?;
        Ok(self.into_bytes())
    }
}

impl fmt::Display for CommonMessageHeader {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "M2PA Common Message Header [version={}, spare={}, class={}, type={}, length={}]",
            self.version(),
            self.spare(),
            self.message_class(),
            self.message_type(),
            self.message_length()
        )
    }
}

/// M2PA-specific message header (8 bytes).
///
/// ```ignore
/// 0                   1                   2                   3
/// 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
/// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
/// |     unused    |                      BSN                      |
/// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
/// |     unused    |                      FSN                      |
/// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
/// ```
///
/// See <https://www.rfc-editor.org/rfc/rfc4165.html#section-2.2>
#[bitfield]
pub struct M2PAHeader {
    #[skip]
    __: B8,
    /// Backward Sequence Number: FSN of the message last received from peer.
    pub bsn: B24,
    #[skip]
    __: B8,
    /// Forward Sequence Number: sequence number of this User Data message.
    pub fsn: B24,
}

impl Default for M2PAHeader {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for M2PAHeader {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "M2PA Header [bsn={}, fsn={}]", self.bsn(), self.fsn())
    }
}

/// Decode an 8-byte array into an M2PAHeader.
pub fn decode_m2pa_header(encoded: [u8; 8]) -> M2PAHeader {
    M2PAHeader::from_bytes(encoded)
}

/// Encode an M2PAHeader into an 8-byte array.
pub fn encode_m2pa_header(m2pa_header: M2PAHeader) -> [u8; 8] {
    m2pa_header.into_bytes()
}

/// A complete M2PA message (header + body).
#[derive(Debug, Clone)]
pub enum M2paMessage {
    /// User Data message carrying an MTP3 MSU.
    UserData {
        bsn: u32,
        fsn: u32,
        message: UserDataMessage,
    },
    /// Link Status message for alignment/proving.
    LinkStatus {
        bsn: u32,
        fsn: u32,
        message: LinkStatusMessage,
    },
}

impl M2paMessage {
    /// Decode a complete M2PA message from raw bytes.
    pub fn decode(bytes: &[u8]) -> Result<Self, M2paError> {
        if bytes.len() < 16 {
            return Err(M2paError::TooShort {
                expected: 16,
                actual: bytes.len(),
            });
        }

        let mut cmh_bytes = [0u8; 8];
        cmh_bytes.copy_from_slice(&bytes[0..8]);
        let cmh = CommonMessageHeader::decode(cmh_bytes)?;

        let mut m2pa_bytes = [0u8; 8];
        m2pa_bytes.copy_from_slice(&bytes[8..16]);
        let m2pa_hdr = decode_m2pa_header(m2pa_bytes);

        let body = &bytes[16..];

        match cmh.message_type() {
            MESSAGE_TYPE_USER_DATA => {
                let user_data = UserDataMessage::decode(body)?;
                Ok(M2paMessage::UserData {
                    bsn: m2pa_hdr.bsn(),
                    fsn: m2pa_hdr.fsn(),
                    message: user_data,
                })
            }
            MESSAGE_TYPE_LINK_STATUS => {
                let link_status = LinkStatusMessage::decode(body)?;
                Ok(M2paMessage::LinkStatus {
                    bsn: m2pa_hdr.bsn(),
                    fsn: m2pa_hdr.fsn(),
                    message: link_status,
                })
            }
            other => Err(M2paError::InvalidMessageType(other)),
        }
    }

    /// Encode a complete M2PA message to bytes.
    pub fn encode(&self) -> Result<Vec<u8>, M2paError> {
        let (msg_type, bsn, fsn, body) = match self {
            M2paMessage::UserData { bsn, fsn, message } => {
                (MESSAGE_TYPE_USER_DATA, *bsn, *fsn, message.encode())
            }
            M2paMessage::LinkStatus { bsn, fsn, message } => {
                (MESSAGE_TYPE_LINK_STATUS, *bsn, *fsn, message.encode().to_vec())
            }
        };

        let total_len = 16 + body.len();

        let cmh = CommonMessageHeader::new()
            .with_version(VERSION)
            .with_message_class(MESSAGE_CLASS_M2PA)
            .with_message_type(msg_type)
            .with_message_length(total_len as u32);

        let m2pa_hdr = M2PAHeader::new().with_bsn(bsn).with_fsn(fsn);

        let mut buf = Vec::with_capacity(total_len);
        buf.extend_from_slice(&cmh.encode()?);
        buf.extend_from_slice(&encode_m2pa_header(m2pa_hdr));
        buf.extend_from_slice(&body);
        Ok(buf)
    }
}

impl fmt::Display for M2paMessage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UserData { bsn, fsn, message } => {
                write!(f, "M2PA User Data [bsn={bsn}, fsn={fsn}, {message}]")
            }
            Self::LinkStatus { bsn, fsn, message } => {
                write!(f, "M2PA Link Status [bsn={bsn}, fsn={fsn}, {message}]")
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decode_common_message_header_correct() {
        let buf: [u8; 8] = [1, 0, 11, 1, 0, 0, 0, 100];
        let cmh = CommonMessageHeader::decode(buf).unwrap();
        assert_eq!(cmh.version(), 1);
        assert_eq!(cmh.spare(), 0);
        assert_eq!(cmh.message_class(), 11);
        assert_eq!(cmh.message_type(), 1);
        assert_eq!(cmh.message_length(), 100);
    }

    #[test]
    fn decode_common_message_header_version_wrong() {
        let buf: [u8; 8] = [0, 0, 11, 1, 0, 0, 0, 100];
        assert!(CommonMessageHeader::decode(buf).is_err());
    }

    #[test]
    fn decode_common_message_header_spare_wrong() {
        let buf: [u8; 8] = [1, 1, 11, 1, 0, 0, 0, 100];
        assert!(CommonMessageHeader::decode(buf).is_err());
    }

    #[test]
    fn decode_common_message_header_message_class_wrong() {
        let buf: [u8; 8] = [1, 0, 12, 1, 0, 0, 0, 100];
        assert!(CommonMessageHeader::decode(buf).is_err());
    }

    #[test]
    fn decode_common_message_header_message_type_wrong() {
        let buf: [u8; 8] = [1, 0, 11, 123, 0, 0, 0, 100];
        assert!(CommonMessageHeader::decode(buf).is_err());
    }

    #[test]
    fn encode_common_message_header_round_trip() {
        let cmh = CommonMessageHeader::new()
            .with_version(1)
            .with_message_class(11)
            .with_message_type(1)
            .with_message_length(20);

        let serialized = cmh.encode().unwrap();
        let parsed = CommonMessageHeader::decode(serialized).unwrap();
        assert_eq!(parsed.version(), 1);
        assert_eq!(parsed.spare(), 0);
        assert_eq!(parsed.message_class(), 11);
        assert_eq!(parsed.message_type(), 1);
        assert_eq!(parsed.message_length(), 20);
    }

    #[test]
    fn m2pa_header_round_trip() {
        let hdr = M2PAHeader::new().with_bsn(12).with_fsn(11);
        let serialized = encode_m2pa_header(hdr);
        assert_eq!(serialized.len(), 8);
        let parsed = decode_m2pa_header(serialized);
        assert_eq!(parsed.bsn(), 12);
        assert_eq!(parsed.fsn(), 11);
    }

    #[test]
    fn decode_link_status_proving_emergency() {
        // Full M2PA Link Status message: Proving Emergency
        // Common Header: version=1, spare=0, class=11, type=2 (Link Status), length=20
        // M2PA Header: BSN=0xFFFFFF, FSN=0xFFFFFF (initial values)
        // Body: state=3 (Proving Emergency)
        let hex_str = "01000b020000001400ffffff00ffffff00000003";
        let bytes: Vec<u8> = hex::decode(hex_str).unwrap();

        let msg = M2paMessage::decode(&bytes).unwrap();
        match msg {
            M2paMessage::LinkStatus { bsn, fsn, message } => {
                assert_eq!(bsn, 0xFFFFFF);
                assert_eq!(fsn, 0xFFFFFF);
                assert_eq!(message.state, LinkState::ProvingEmergency);
            }
            _ => panic!("Expected LinkStatus message"),
        }
    }

    #[test]
    fn encode_user_data_round_trip() {
        let msu = vec![0x83, 0x01, 0x02, 0x03];
        let msg = M2paMessage::UserData {
            bsn: 100,
            fsn: 200,
            message: UserDataMessage::new(1, msu.clone()),
        };

        let encoded = msg.encode().unwrap();
        let decoded = M2paMessage::decode(&encoded).unwrap();

        match decoded {
            M2paMessage::UserData { bsn, fsn, message } => {
                assert_eq!(bsn, 100);
                assert_eq!(fsn, 200);
                assert_eq!(message.priority, 1);
                assert_eq!(message.msu, msu);
            }
            _ => panic!("Expected UserData message"),
        }
    }

    #[test]
    fn encode_link_status_round_trip() {
        let msg = M2paMessage::LinkStatus {
            bsn: 0xFFFFFF,
            fsn: 0xFFFFFF,
            message: LinkStatusMessage::new(LinkState::Ready),
        };

        let encoded = msg.encode().unwrap();
        let decoded = M2paMessage::decode(&encoded).unwrap();

        match decoded {
            M2paMessage::LinkStatus { bsn, fsn, message } => {
                assert_eq!(bsn, 0xFFFFFF);
                assert_eq!(fsn, 0xFFFFFF);
                assert_eq!(message.state, LinkState::Ready);
            }
            _ => panic!("Expected LinkStatus message"),
        }
    }

    #[test]
    fn decode_too_short() {
        let bytes = [0u8; 10];
        assert!(M2paMessage::decode(&bytes).is_err());
    }

    #[test]
    fn message_display() {
        let msg = M2paMessage::LinkStatus {
            bsn: 0,
            fsn: 0,
            message: LinkStatusMessage::new(LinkState::Alignment),
        };
        let s = format!("{msg}");
        assert!(s.contains("Alignment"));
    }
}
