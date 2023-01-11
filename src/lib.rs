use std::fmt;

/*
   2.1.  Common Message Header

   The protocol messages for M2PA require a message header structure
   that contains a version, message class, message type, and message
   length.  The header structure is shown in Figure 5.

       0                   1                   2                   3
       0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
      +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
      |    Version    |     Spare     | Message Class | Message Type  |
      +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
      |                        Message Length                         |
      +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+

                     Figure 5.  Common Message Header
 */

#[repr(C, packed)]
#[derive(Debug, Copy, Clone)]
pub struct CommonMessageHeader {
    /*
       2.1.1.  Version

       The version field contains the version of M2PA.  The supported
       versions are:

            Value
          (decimal)  Version
          ---------  -------
              1      Release 1.0 of M2PA protocol
     */
    pub version: u8,
    /*
       2.1.2.  Spare

       The Spare field SHOULD be set to all zeroes (0's) by the sender and
       ignored by the receiver.  The Spare field SHOULD NOT be used for
       proprietary information.
     */
    pub spare: u8,

    /*
       2.1.3.  Message Class

       The following List contains the valid Message Classes:

            Value
          (decimal)  Message Class
          ---------  -------------
             11      M2PA Messages

       Other values are invalid for M2PA.
     */
    pub message_class: u8,

    /*
        2.1.4.  Message Type

        The following list contains the message types for the defined
        messages.

            Value
          (decimal)  Message Type
          ---------  -------------
              1      User Data
              2      Link Status

        Other values are invalid.
     */
    pub message_type: u8,

    /*
        2.1.5.  Message Length

        The Message Length defines the length of the message in octets,
        including the Common Header.
     */
    pub message_length: u32
}

impl fmt::Display for CommonMessageHeader {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let message_length = self.message_length; // To prevent https://github.com/rust-lang/rust/issues/82523
        write!(f, "M2PA Common Message Header [version={}, spare={}, class={}, type={}, length={}]", self.version, self.spare, self.message_class, self.message_type, message_length)
    }
}
/// Decodes an array u8 into a CommonMessageHeader struct
///
/// # Arguments
///
/// * `encoded` - An array of 8 bytes (u8) containing the data for the Common Message Header
///
/// # Examples
///
/// ```
/// let encoded: &[u8] = &[1, 0, 11, 1, 100, 0, 0, 0];
/// let decoded = unsafe { m2pa::decode_common_message_header(encoded) };
/// ```
pub unsafe fn decode_common_message_header(encoded: &[u8]) -> &CommonMessageHeader {
    assert!(encoded.len() == 8, "Common Message Header should be 8 bytes");

    let (head, body, _tail) = unsafe { encoded.align_to::<CommonMessageHeader>() };
    assert!(head.is_empty(), "Data was not aligned");
    let common_message_header = &body[0];

    // Message validation against RFC 4165
    assert!(common_message_header.version == 1); // Version should alwyas be 1
    assert!(common_message_header.spare == 0); // Spare should be set to 0
    assert!(common_message_header.message_class == 11); // M2PA message
    assert!(common_message_header.message_type == 1 || common_message_header.message_type == 2); // Either 1 - User data or 2 - Link Status
    assert!(common_message_header.message_length < 240); // We expect header + MSU to be less than 240

    let len = common_message_header.message_length; // To prevent https://github.com/rust-lang/rust/issues/82523

    println!("{} {}", common_message_header, len);

    return common_message_header;
}

/// Encodes a CommonMessageHeader struct into an array of u8
///
/// # Arguments
///
/// * `header` - The CommonMessageHeader to encode
///
/// # Examples
///
/// ```
/// let m2pa_header = m2pa::CommonMessageHeader {
///     version: 1,
///     spare: 0,
///     message_class: 11,
///     message_type: 1,
///     message_length: 239
/// };
/// 
/// let encoded = unsafe { m2pa::encode_common_message_header(&m2pa_header) };
/// ```
pub unsafe fn encode_common_message_header(header: &CommonMessageHeader) -> &[u8] {
    ::std::slice::from_raw_parts(
        (header as *const CommonMessageHeader) as *const u8,
        ::std::mem::size_of::<CommonMessageHeader>(),
    )
}

#[cfg(test)]
mod common_message_header_tests {
    use super::*;

    //
    // decode_common_message_header tests
    // 
    #[test]
    fn decode_common_message_header_correct() {
        let buf: &[u8] = &[1, 0, 11, 1, 100, 0, 0, 0];
        unsafe { decode_common_message_header(buf) };
    }

    #[test]
    #[should_panic(expected = "Common Message Header should be 8 bytes")]
    fn decode_common_message_header_not_aligned_too_short() {
        let buf: &[u8] = &[1, 0, 11, 1];
        unsafe { decode_common_message_header(buf) };
    }

    #[test]
    #[should_panic(expected = "Common Message Header should be 8 bytes")]
    fn decode_common_message_header_not_aligned_too_big() {
        let buf: &[u8] = &[1, 0, 11, 1, 100, 0, 0, 0, 0, 0, 0];
        unsafe { decode_common_message_header(buf) };
    }

    #[test]
    #[should_panic(expected = "assertion failed: common_message_header.version == 1")]
    fn decode_common_message_header_version_wrong() {
        let buf: &[u8] = &[0, 0, 11, 1, 100, 0, 0, 0];
        unsafe { decode_common_message_header(buf) };
    }

    #[test]
    #[should_panic(expected = "assertion failed: common_message_header.spare == 0")]
    fn decode_common_message_header_spare_wrong() {
        let buf: &[u8] = &[1, 1, 11, 1, 100, 0, 0, 0];
        unsafe { decode_common_message_header(buf) };
    }

    #[test]
    #[should_panic(expected = "assertion failed: common_message_header.message_class == 11")]
    fn decode_common_message_header_message_class_wrong() {
        let buf: &[u8] = &[1, 0, 12, 1, 100, 0, 0, 0];
        unsafe { decode_common_message_header(buf) };
    }

    #[test]
    #[should_panic(expected = "assertion failed: common_message_header.message_type == 1")]
    fn decode_common_message_header_message_type_wrong() {
        let buf: &[u8] = &[1, 0, 11, 123, 100, 0, 0, 0];
        unsafe { decode_common_message_header(buf) };
    }

    #[test]
    #[should_panic(expected = "assertion failed: common_message_header.message_length < 240")]
    fn decode_common_message_header_message_length_wrong() {
        let buf: &[u8] = &[1, 0, 11, 1, 0, 0, 0, 245];
        unsafe { decode_common_message_header(buf) };
    }

    //
    // encode_common_message_header tests
    // 

    #[test]
    fn encode_common_message_header_correct() {
        let m2pa_header = CommonMessageHeader {
            version: 1,
            spare: 0,
            message_class: 11,
            message_type: 1,
            message_length: 239
        };

        let serialized = unsafe { encode_common_message_header(&m2pa_header) };

        assert_eq!(serialized.len(), 8); // Common Message Header should be 8 bytes

        // By parsing the message again we make sure that it was encoded directly as it should be 1:1 with the original
        let parsed = unsafe { decode_common_message_header(serialized) };

        assert_eq!(parsed.version, 1);
        assert_eq!(parsed.spare, 0);
        assert_eq!(parsed.message_class, 11);
        assert_eq!(parsed.message_type, 1);

        let len = parsed.message_length; // To prevent https://github.com/rust-lang/rust/issues/82523
        assert_eq!(len, 239);
    }

}
