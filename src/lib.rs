#[macro_use]
extern crate slice_as_array;
extern crate matches;

use std::fmt;
use modular_bitfield_msb::{bitfield, specifiers::{B8, B24, B32}};

// Taken from https://stackoverflow.com/questions/53124930/how-do-you-test-for-a-specific-rust-error/53124931
macro_rules! assert_err {
    ($expression:expr, $($pattern:tt)+) => {
        match $expression {
            $($pattern)+ => (),
            ref e => panic!("expected `{}` but got `{:?}`", stringify!($($pattern)+), e),
        }
    }
}

/// The protocol messages for M2PA require a message header structure
/// that contains a version, message class, message type, and message
/// length. The header structure is shown in Figure 5.
/// ```ignore
/// 0                   1                   2                   3
/// 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
/// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
/// |    Version    |     Spare     | Message Class | Message Type  |
/// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
/// |                        Message Length                         |
/// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
/// 
/// Figure 5.  Common Message Header
/// ```        
/// 
/// See <https://www.rfc-editor.org/rfc/rfc4165.html#section-2.1>
/// 
/// 
#[derive(Clone, Copy)]
#[bitfield]
pub struct CommonMessageHeader {
    /// The version field contains the version of M2PA.  The supported
    /// versions are:
    /// ```ignore
    ///     Value
    ///   (decimal)  Version
    ///   ---------  -------
    ///       1      Release 1.0 of M2PA protocol
    /// ```
    pub version: B8,
    
    /// The Spare field SHOULD be set to all zeroes (0's) by the sender and
    /// ignored by the receiver.  The Spare field SHOULD NOT be used for
    /// proprietary information.
    #[skip(setters)]
    pub spare: B8,

    /// The following List contains the valid Message Classes:
    /// ```ignore
    ///     Value
    ///   (decimal)  Message Class
    ///   ---------  -------------
    ///      11      M2PA Messages
    /// ```
    /// Other values are invalid for M2PA.
    pub message_class: B8,

    /// The following list contains the message types for the defined messages.
    /// ```ignore
    ///     Value
    ///   (decimal)  Message Type
    ///   ---------  -------------
    ///       1      User Data
    ///       2      Link Status
    /// ```
    ///Other values are invalid.
    pub message_type: B8,

    /// The Message Length defines the length of the message in octets,
    /// including the Common Header.
    pub message_length: B32
}

impl CommonMessageHeader {

    /// Message validation against RFC 4165
    /// 
    /// # Arguments
    /// 
    /// * `header` - Common Message Header to validate against RFC 4165 and sensible defaults enforced by this library
    /// 
    /// # Examples
    /// 
    /// ```
    /// let common_message_header = m2pa::CommonMessageHeader::new()
    /// .with_version(1)
    /// .with_message_class(11)
    /// .with_message_type(1)
    /// .with_message_length(20);
    /// 
    /// match m2pa::CommonMessageHeader::validate(common_message_header) {
    ///     Ok(valid) => println!("Common Message Header valid! => {}", valid),
    ///     Err(e) => panic!("Common Message Header not valid, {}", e)
    /// }
    /// ```
    pub fn validate(header: CommonMessageHeader) -> Result<CommonMessageHeader, str&> { 
        if header.version() != 1 { return Err(String::from("Version should always be 1")) }
        if header.spare() != 0 { return Err(String::from("Spare should be set to 0")) }
        if header.message_class() != 11  { return Err(String::from("Message class invalid")) }
        if header.message_type() != 1 && header.message_type() != 2 { return Err(String::from("Message type can only be User Data (1) or Link Status (2)")) }
        if header.message_length() > 240 { return Err(String::from("We expect header + MSU to be less than 240")) }
        Ok(header)
    }

    /// Decodes an validates an array u8 into a CommonMessageHeader struct
    ///
    /// # Arguments
    ///
    /// * `bytes` - An array of 8 bytes (u8) containing the data for the Common Message Header
    ///
    /// # Examples
    ///
    /// ```
    /// let encoded: [u8;8] = [1, 0, 11, 1, 100, 0, 0, 0];
    /// let decoded = m2pa::CommonMessageHeader::decode(encoded);
    /// ```
    pub fn decode(bytes: [u8; 8]) -> Result<CommonMessageHeader, String> {
        // As the compiler ensures that we feed an array of 8 bytes there we only need to introspect if the decoded values are sensible
        CommonMessageHeader::validate(CommonMessageHeader::from_bytes(bytes))
    }

    //
    pub fn encode(self) -> Result<[u8; 8], String> {
        match CommonMessageHeader::validate(self) {
            Ok(valid) => Ok(valid.into_bytes()),
            Err(e) => Err(e)
        }
    }
}

impl fmt::Display for CommonMessageHeader {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "M2PA Common Message Header [version={}, spare={}, class={}, type={}, length={}]", self.version(), self.spare(), self.message_class(), self.message_type(), self.message_length())
    }
}

#[cfg(test)]
mod common_message_header_tests {
    use core::panic;

    use super::*;

    //
    // decode_common_message_header tests
    // 
    #[test]
    fn decode_common_message_header_correct() {
        let buf: [u8;8] = [1, 0, 11, 1, 0, 0, 0, 100];
        CommonMessageHeader::decode(buf)
    }

    #[test]
    #[should_panic(expected = "assertion failed: common_message_header.version() == 1")]
    fn decode_common_message_header_version_wrong() {
        let buf: [u8;8] = [0, 0, 11, 1, 100, 0, 0, 0];
        assert_err!(CommonMessageHeader::decode(buf), Error(String::from("dingen")));
    }

    #[test]
    #[should_panic(expected = "assertion failed: common_message_header.spare() == 0")]
    fn decode_common_message_header_spare_wrong() {
        let buf: [u8;8] = [1, 1, 11, 1, 100, 0, 0, 0];
        CommonMessageHeader::decode(buf);
    }

    #[test]
    #[should_panic(expected = "assertion failed: common_message_header.message_class() == 11")]
    fn decode_common_message_header_message_class_wrong() {
        let buf: [u8;8] = [1, 0, 12, 1, 100, 0, 0, 0];
        CommonMessageHeader::decode(buf);
    }

    #[test]
    #[should_panic(expected = "assertion failed: common_message_header.message_type() == 1")]
    fn decode_common_message_header_message_type_wrong() {
        let buf: [u8;8] = [1, 0, 11, 123, 100, 0, 0, 0];
        CommonMessageHeader::decode(buf);
    }

    #[test]
    #[should_panic(expected = "assertion failed: common_message_header.message_length() < 240")]
    fn decode_common_message_header_message_length_wrong() {
        let buf: [u8;8] = [1, 0, 11, 1, 0, 0, 0, 245];
        CommonMessageHeader::decode(buf);
    }

    //
    // encode_common_message_header tests
    // 

    #[test]
    fn encode_common_message_header_correct() {
        let common_message_header = CommonMessageHeader::new()
            .with_version(1)
            .with_message_class(11)
            .with_message_type(1)
            .with_message_length(20);

        match common_message_header.encode() {
            Ok(serialized) => {
                println!("{:?}", serialized);
                // By parsing the message again we make sure that it was encoded directly as it should be 1:1 with the original
                match CommonMessageHeader::decode(serialized) {
                    Ok(parsed) => {
                        assert_eq!(parsed.version(), 1);
                        assert_eq!(parsed.spare(), 0);
                        assert_eq!(parsed.message_class(), 11);
                        assert_eq!(parsed.message_type(), 1);
                        assert_eq!(parsed.message_length(), 20);
                    },
                    Err(e) => panic!("Could not decode common message header {}", e)
                }
            },
            Err(e) => panic!("Could not encode common message header {}", e)
        }

        
    }

    // TODO test with wrong message class etc

}

/// All protocol messages for M2PA require an M2PA-specific header.  The
/// header structure is shown in Figure 6.
/// 
/// ```ignore
/// 0                   1                   2                   3
/// 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
/// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
/// |     unused    |                      BSN                      |
/// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
/// |     unused    |                      FSN                      |
/// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
/// 
///             Figure 6.  M2PA-specific Message Header
/// ```
/// 
/// The FSN and BSN values range from 0 to 16,777,215.
/// 
/// See <https://www.rfc-editor.org/rfc/rfc4165.html#section-2.2>
#[bitfield]
pub struct M2PAHeader {
    #[skip] __: B8,
    /// This is the FSN of the message last received from the peer.
    pub bsn: B24,
    #[skip] __: B8,
    ///  This is the M2PA sequence number of the User Data message being sent.
    pub fsn: B24
}

impl fmt::Display for M2PAHeader {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "M2PA Header [bsn={}, fsn={}]", self.bsn(), self.fsn())
    }
}

/// Decodes an array u8 into a M2PAHeader struct
///
/// # Arguments
///
/// * `encoded` - An array of 8 bytes (u8) containing the data for the M2PA header
///
/// # Examples
///
/// ```
/// let encoded: [u8;8] = &[0, 11, 0, 0, 0, 12, 0, 0];
/// let decoded = m2pa::decode_m2pa_header(encoded);
/// ```
pub fn decode_m2pa_header(encoded: [u8;8]) -> M2PAHeader {
    assert!(encoded.len() == 8, "M2PA header should be 8 bytes");
    M2PAHeader::from_bytes(encoded)
}

/// Encodes a M2PAHeader struct into an array of u8
///
/// # Arguments
///
/// * `header` - The M2PAHeader to encode
///
/// # Examples
///
/// ```
/// let m2pa_header = m2pa::M2PAHeader::new().with_bsn(12).with_fsn(11);
/// let encoded = m2pa::encode_m2pa_header(m2pa_header);
/// ```
pub fn encode_m2pa_header(m2pa_header: M2PAHeader) -> [u8;8] {
    m2pa_header.into_bytes()
}

#[cfg(test)]
mod m2pa_header_tests {
    use super::*;

    //
    // decode_m2pa_header tests
    // 
    #[test]
    fn decode_m2pa_header_correct() {
        let buf: [u8; 8] = [1, 0, 11, 1, 0, 0, 0, 12];
       decode_m2pa_header(buf);
    }

    //
    // encode_m2pa_header tests
    // 

    #[test]
    fn encode_m2pa_header_correct() {
        let m2pa_header = M2PAHeader::new().with_bsn(12).with_fsn(11);

        let serialized = encode_m2pa_header(m2pa_header);

        println!("{:?}", serialized);

        assert_eq!(serialized.len(), 8); // M2PA Header should be 8 bytes

        // By parsing the message again we make sure that it was encoded directly as it should be 1:1 with the original
        let parsed = decode_m2pa_header(serialized);

        assert_eq!(parsed.bsn(), 12);
        assert_eq!(parsed.fsn(), 11);
    }

}

#[cfg(test)]
mod m2pa_tests {

    fn slice_as_hash(xs: &[u8]) -> &[u8; 8] {
        println!("{:?}", xs);
        slice_as_array!(xs, [u8; 8]).expect("bad hash length")
    }

    #[test]
    fn decode_link_status_proving_emergency() {
        let m2pa_packet_hex = "01000b020000001400ffffff00ffffff00000003";
        let decoded: [u8; 20] = hex::FromHex::from_hex(m2pa_packet_hex).expect("Decoding failed");

        let common_message_header_bytes = slice_as_hash(&decoded[0..8]);
        let common_message_header = decode_common_message_header(*common_message_header_bytes);

        let m2pa_header_bytes = slice_as_hash(&decoded[8..16]);
        let m2pa_header = decode_m2pa_header(*m2pa_header_bytes);

        println!("{}", common_message_header);
        println!("{}", m2pa_header);
        println!("{:?}", decoded);
    }
}

