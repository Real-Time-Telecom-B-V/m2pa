//! Decode an M2PA packet from a hex dump.
//!
//! This example decodes a real M2PA Link Status message
//! (Proving Emergency) captured from a Wireshark trace.

use m2pa::M2paMessage;

fn main() {
    // M2PA Link Status: Proving Emergency
    // Common Header: version=1, spare=0, class=11, type=2, length=20
    // M2PA Header: BSN=0xFFFFFF, FSN=0xFFFFFF
    // Body: state=3 (Proving Emergency)
    let hex_str = "01000b020000001400ffffff00ffffff00000003";
    let bytes: Vec<u8> = hex::decode(hex_str).expect("Invalid hex string");

    println!("Raw bytes ({} bytes): {:02x?}", bytes.len(), bytes);
    println!();

    match M2paMessage::decode(&bytes) {
        Ok(msg) => println!("Decoded: {msg}"),
        Err(e) => eprintln!("Decode error: {e}"),
    }
}
