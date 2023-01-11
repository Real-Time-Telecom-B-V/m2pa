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
    // This trait requires `fmt` with this exact signature.
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        // Write strictly the first element into the supplied output
        // stream: `f`. Returns `fmt::Result` which indicates whether the
        // operation succeeded or failed. Note that `write!` uses syntax which
        // is very similar to `println!`.
        write!(f, "M2PA Common Message Header [version={}, spare={}, class={}, type={}, length={}]", self.version, self.spare, self.message_class, self.message_type, self.message_length)
    }
}

pub fn add(left: usize, right: usize) -> usize {
    left + right
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let result = add(2, 2);
        assert_eq!(result, 4);
    }
}
