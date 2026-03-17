//! Integration tests — M2PA packet decoding from known hex dumps.

use m2pa::*;

/// Decode a real M2PA Link Status message: Proving Emergency.
/// Captured from a Wireshark SS7 trace.
///
/// Hex: 01000b020000001400ffffff00ffffff00000003
///
/// Breakdown:
///   Common Header (8 bytes):
///     01          Version = 1
///     00          Spare = 0
///     0b          Message Class = 11 (M2PA)
///     02          Message Type = 2 (Link Status)
///     00000014    Message Length = 20
///   M2PA Header (8 bytes):
///     00ffffff    BSN = 0xFFFFFF (initial)
///     00ffffff    FSN = 0xFFFFFF (initial)
///   Body (4 bytes):
///     00000003    State = 3 (Proving Emergency)
#[test]
fn decode_real_link_status_proving_emergency() {
    let hex_str = "01000b020000001400ffffff00ffffff00000003";
    let bytes = hex::decode(hex_str).unwrap();

    let msg = M2paMessage::decode(&bytes).unwrap();
    match msg {
        M2paMessage::LinkStatus { bsn, fsn, message } => {
            assert_eq!(bsn, 0xFFFFFF);
            assert_eq!(fsn, 0xFFFFFF);
            assert_eq!(message.state, LinkState::ProvingEmergency);
        }
        _ => panic!("Expected LinkStatus"),
    }
}

/// Encode a Link Status Ready and verify hex output.
#[test]
fn encode_link_status_ready() {
    let msg = M2paMessage::LinkStatus {
        bsn: 0xFFFFFF,
        fsn: 0xFFFFFF,
        message: LinkStatusMessage::new(LinkState::Ready),
    };
    let encoded = msg.encode().unwrap();

    // Verify the state field at the end
    let state_bytes = &encoded[16..20];
    assert_eq!(state_bytes, &[0x00, 0x00, 0x00, 0x04]); // Ready = 4
}

/// Link Status Alignment message.
#[test]
fn link_status_alignment() {
    let msg = M2paMessage::LinkStatus {
        bsn: 0,
        fsn: 0,
        message: LinkStatusMessage::new(LinkState::Alignment),
    };
    let encoded = msg.encode().unwrap();
    let decoded = M2paMessage::decode(&encoded).unwrap();

    match decoded {
        M2paMessage::LinkStatus { bsn, fsn, message } => {
            assert_eq!(bsn, 0);
            assert_eq!(fsn, 0);
            assert_eq!(message.state, LinkState::Alignment);
        }
        _ => panic!("Expected LinkStatus"),
    }
}

/// User Data message with MTP3 MSU payload.
#[test]
fn user_data_with_msu() {
    let msu = vec![
        0x83, // SIO: NI=National, SI=SCCP
        0x01, 0x00, 0x00, 0x00, // Routing label (ITU)
        0x09, 0x00, 0x03, 0x05, // SCCP UDT header
    ];

    let msg = M2paMessage::UserData {
        bsn: 100,
        fsn: 101,
        message: UserDataMessage::new(0, msu.clone()),
    };

    let encoded = msg.encode().unwrap();
    let decoded = M2paMessage::decode(&encoded).unwrap();

    match decoded {
        M2paMessage::UserData { bsn, fsn, message } => {
            assert_eq!(bsn, 100);
            assert_eq!(fsn, 101);
            assert_eq!(message.priority, 0);
            assert_eq!(message.msu, msu);
        }
        _ => panic!("Expected UserData"),
    }
}

/// State machine: full alignment sequence.
#[test]
fn state_machine_full_alignment() {
    let mut sm = M2paStateMachine::new();
    assert_eq!(sm.state(), M2paState::OutOfService);

    // Start alignment
    sm.start();
    assert_eq!(sm.state(), M2paState::NotAligned);

    // Peer sends Alignment
    sm.on_link_status(LinkState::Alignment);
    assert_eq!(sm.state(), M2paState::Aligned);

    // Peer sends Proving Normal
    sm.on_link_status(LinkState::ProvingNormal);
    assert_eq!(sm.state(), M2paState::Proving);

    // Peer sends Ready
    sm.on_link_status(LinkState::Ready);
    assert_eq!(sm.state(), M2paState::AlignedReady);

    // Peer sends Ready again → In Service!
    sm.on_link_status(LinkState::Ready);
    assert_eq!(sm.state(), M2paState::InService);
}

/// Decode M2PA Link Status Ready from known wire bytes.
#[test]
fn decode_link_status_ready_wire() {
    let hex_str = "01000b020000001400ffffff00ffffff00000004";
    let bytes = hex::decode(hex_str).unwrap();
    let msg = M2paMessage::decode(&bytes).unwrap();
    match msg {
        M2paMessage::LinkStatus { bsn, fsn, message } => {
            assert_eq!(bsn, 0xFFFFFF);
            assert_eq!(fsn, 0xFFFFFF);
            assert_eq!(message.state, LinkState::Ready);
        }
        _ => panic!("Expected LinkStatus Ready"),
    }
}

/// Decode M2PA Link Status Alignment from known wire bytes.
#[test]
fn decode_link_status_alignment_wire() {
    let hex_str = "01000b020000001400ffffff00ffffff00000001";
    let bytes = hex::decode(hex_str).unwrap();
    let msg = M2paMessage::decode(&bytes).unwrap();
    match msg {
        M2paMessage::LinkStatus { message, .. } => {
            assert_eq!(message.state, LinkState::Alignment);
        }
        _ => panic!("Expected LinkStatus Alignment"),
    }
}

/// Decode M2PA Link Status Processor Outage from known wire bytes.
#[test]
fn decode_link_status_processor_outage_wire() {
    let hex_str = "01000b020000001400ffffff00ffffff00000005";
    let bytes = hex::decode(hex_str).unwrap();
    let msg = M2paMessage::decode(&bytes).unwrap();
    match msg {
        M2paMessage::LinkStatus { message, .. } => {
            assert_eq!(message.state, LinkState::ProcessorOutage);
        }
        _ => panic!("Expected LinkStatus ProcessorOutage"),
    }
}

/// All 8 Link Status states from wire format.
#[test]
fn decode_all_link_status_states_from_wire() {
    let states = [
        ("01000b020000001400ffffff00ffffff00000001", LinkState::Alignment),
        ("01000b020000001400ffffff00ffffff00000002", LinkState::ProvingNormal),
        ("01000b020000001400ffffff00ffffff00000003", LinkState::ProvingEmergency),
        ("01000b020000001400ffffff00ffffff00000004", LinkState::Ready),
        ("01000b020000001400ffffff00ffffff00000005", LinkState::ProcessorOutage),
        ("01000b020000001400ffffff00ffffff00000006", LinkState::ProcessorRecovered),
        ("01000b020000001400ffffff00ffffff00000007", LinkState::Busy),
        ("01000b020000001400ffffff00ffffff00000008", LinkState::BusyEnded),
    ];

    for (hex_str, expected_state) in states {
        let bytes = hex::decode(hex_str).unwrap();
        let msg = M2paMessage::decode(&bytes).unwrap();
        match msg {
            M2paMessage::LinkStatus { message, .. } => {
                assert_eq!(message.state, expected_state, "Failed for {hex_str}");
            }
            _ => panic!("Expected LinkStatus for {hex_str}"),
        }
    }
}

/// Common header validation rejects invalid values.
#[test]
fn header_validation_errors() {
    // Wrong version
    let buf = [2, 0, 11, 1, 0, 0, 0, 20];
    assert!(CommonMessageHeader::decode(buf).is_err());

    // Wrong class
    let buf = [1, 0, 12, 1, 0, 0, 0, 20];
    assert!(CommonMessageHeader::decode(buf).is_err());

    // Wrong type (not 1 or 2)
    let buf = [1, 0, 11, 3, 0, 0, 0, 20];
    assert!(CommonMessageHeader::decode(buf).is_err());

    // Non-zero spare
    let buf = [1, 1, 11, 1, 0, 0, 0, 20];
    assert!(CommonMessageHeader::decode(buf).is_err());
}
