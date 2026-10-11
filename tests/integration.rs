//! Integration tests — M2PA message encode/decode against RFC 4165 wire vectors.
//!
//! Every vector here is derived straight from the spec, not captured traffic. A
//! Link Status message is pure control signalling — the common header
//! (version/class/type/length), the BSN/FSN sequence words, and a 4-byte state —
//! so its bytes are fully determined by RFC 4165 §2/§3 and carry no user data.
//! The User Data vectors use synthetic MTP3 payloads. Where a test pins a hex
//! string, an accompanying encode test proves the crate's encoder reproduces it.

use m2pa::*;

/// Build the 20-byte wire form of a Link Status message with the given state,
/// using the RFC's initial BSN/FSN sentinel (`0xFFFFFF`). Hand-assembled from
/// the spec so the decode tests below have an independent oracle.
fn link_status_wire(state_value: u32) -> Vec<u8> {
    let mut v = Vec::with_capacity(20);
    v.extend_from_slice(&[0x01, 0x00, 0x0b, 0x02]); // version, spare, class=11, type=2
    v.extend_from_slice(&20u32.to_be_bytes()); // message length
    v.extend_from_slice(&0x00FF_FFFFu32.to_be_bytes()); // BSN
    v.extend_from_slice(&0x00FF_FFFFu32.to_be_bytes()); // FSN
    v.extend_from_slice(&state_value.to_be_bytes()); // state
    v
}

// ── Link Status ─────────────────────────────────────────────────────────────

#[test]
fn decode_link_status_proving_emergency() {
    // RFC 4165 wire form of a Proving Emergency (state = 3) Link Status.
    let bytes = hex::decode("01000b020000001400ffffff00ffffff00000003").unwrap();
    assert_eq!(bytes, link_status_wire(3)); // hex string == hand-assembled oracle

    let msg = M2paMessage::decode(&bytes).unwrap();
    match msg {
        M2paMessage::LinkStatus { bsn, fsn, message } => {
            assert_eq!(bsn, 0xFFFFFF);
            assert_eq!(fsn, 0xFFFFFF);
            assert_eq!(message.state, LinkState::ProvingEmergency);
        }
        _ => panic!("expected LinkStatus"),
    }
}

#[test]
fn encode_link_status_ready_matches_wire() {
    let msg = M2paMessage::LinkStatus {
        bsn: 0xFFFFFF,
        fsn: 0xFFFFFF,
        message: LinkStatusMessage::new(LinkState::Ready),
    };
    // The encoder reproduces the RFC wire form exactly (state Ready = 4).
    assert_eq!(msg.encode().unwrap(), link_status_wire(4));
}

#[test]
fn all_link_states_round_trip_and_match_wire() {
    let cases = [
        (1u32, LinkState::Alignment),
        (2, LinkState::ProvingNormal),
        (3, LinkState::ProvingEmergency),
        (4, LinkState::Ready),
        (5, LinkState::ProcessorOutage),
        (6, LinkState::ProcessorRecovered),
        (7, LinkState::Busy),
        (8, LinkState::BusyEnded),
    ];
    for (value, state) in cases {
        // decode(wire) → state
        let decoded = M2paMessage::decode(&link_status_wire(value)).unwrap();
        match decoded {
            M2paMessage::LinkStatus { message, .. } => {
                assert_eq!(message.state, state, "decode failed for state {value}")
            }
            _ => panic!("expected LinkStatus for state {value}"),
        }
        // encode(state) → wire
        let encoded = M2paMessage::LinkStatus {
            bsn: 0xFFFFFF,
            fsn: 0xFFFFFF,
            message: LinkStatusMessage::new(state),
        }
        .encode()
        .unwrap();
        assert_eq!(
            encoded,
            link_status_wire(value),
            "encode failed for {state}"
        );
    }
}

#[test]
fn decode_rejects_unknown_link_state() {
    // RFC 4165 2.3.2 defines 1 to 9; 0 and 10 are not states.
    for value in [0, 10] {
        let err = M2paMessage::decode(&link_status_wire(value)).unwrap_err();
        assert!(matches!(err, M2paError::InvalidLinkStatus(v) if v == value));
    }
}

// ── User Data ───────────────────────────────────────────────────────────────

#[test]
fn user_data_with_synthetic_msu_round_trips() {
    // Synthetic MTP3 MSU: SIO + a fabricated routing label + SCCP-ish header.
    let msu = vec![
        0x83, // SIO: NI=National, SI=SCCP
        0x01, 0x00, 0x00, 0x00, // routing label (fabricated point codes)
        0x09, 0x00, 0x03, 0x05, // SCCP UDT-ish header
    ];
    let msg = M2paMessage::UserData {
        bsn: 100,
        fsn: 101,
        message: UserDataMessage::new(2, msu.clone()),
    };

    let decoded = M2paMessage::decode(&msg.encode().unwrap()).unwrap();
    match decoded {
        M2paMessage::UserData { bsn, fsn, message } => {
            assert_eq!(bsn, 100);
            assert_eq!(fsn, 101);
            assert_eq!(message.priority, 2);
            assert_eq!(message.msu, msu);
        }
        _ => panic!("expected UserData"),
    }
}

#[test]
fn user_data_empty_msu_round_trips() {
    let msg = M2paMessage::UserData {
        bsn: 0,
        fsn: 1,
        message: UserDataMessage::new(0, Vec::new()),
    };
    let decoded = M2paMessage::decode(&msg.encode().unwrap()).unwrap();
    match decoded {
        M2paMessage::UserData { message, .. } => assert!(message.msu.is_empty()),
        _ => panic!("expected UserData"),
    }
}

#[test]
fn user_data_large_msu_round_trips() {
    let msu: Vec<u8> = (0..272u32).map(|i| i as u8).collect(); // an MTP3-sized MSU
    let msg = M2paMessage::UserData {
        bsn: 0x0012_3456,
        fsn: 0x0012_3457,
        message: UserDataMessage::new(3, msu.clone()),
    };
    let decoded = M2paMessage::decode(&msg.encode().unwrap()).unwrap();
    match decoded {
        M2paMessage::UserData { message, .. } => assert_eq!(message.msu, msu),
        _ => panic!("expected UserData"),
    }
}

// ── Common header validation ────────────────────────────────────────────────

#[test]
fn header_validation_errors() {
    // Wrong version.
    assert!(CommonMessageHeader::decode([2, 0, 11, 1, 0, 0, 0, 20]).is_err());
    // Wrong class.
    assert!(CommonMessageHeader::decode([1, 0, 12, 1, 0, 0, 0, 20]).is_err());
    // Wrong type (not 1 or 2).
    assert!(CommonMessageHeader::decode([1, 0, 11, 3, 0, 0, 0, 20]).is_err());
    // Non-zero spare.
    assert!(CommonMessageHeader::decode([1, 1, 11, 1, 0, 0, 0, 20]).is_err());
}

#[test]
fn decode_rejects_truncated_message() {
    // Anything shorter than the two 8-byte headers can't be a message.
    assert!(matches!(
        M2paMessage::decode(&[0x01, 0x00, 0x0b, 0x02]),
        Err(M2paError::TooShort { .. })
    ));
}

// ── State machine ───────────────────────────────────────────────────────────

#[test]
fn state_machine_full_alignment() {
    let mut sm = M2paStateMachine::new();
    assert_eq!(sm.state(), M2paState::OutOfService);

    assert!(sm.start());
    assert_eq!(sm.state(), M2paState::NotAligned);

    sm.on_link_status(LinkState::Alignment);
    assert_eq!(sm.state(), M2paState::Aligned);

    sm.on_link_status(LinkState::ProvingNormal);
    assert_eq!(sm.state(), M2paState::Proving);

    sm.on_link_status(LinkState::Ready);
    assert_eq!(sm.state(), M2paState::AlignedReady);

    sm.on_link_status(LinkState::Ready);
    assert_eq!(sm.state(), M2paState::InService);
}

#[test]
fn state_machine_processor_outage_recovery() {
    let mut sm = M2paStateMachine::new();
    sm.start();
    sm.on_link_status(LinkState::Alignment);
    sm.on_link_status(LinkState::ProvingNormal);
    sm.on_link_status(LinkState::Ready);
    sm.on_link_status(LinkState::Ready);
    assert_eq!(sm.state(), M2paState::InService);

    sm.on_link_status(LinkState::ProcessorOutage);
    assert_eq!(sm.state(), M2paState::AlignedReady);

    sm.on_link_status(LinkState::ProcessorRecovered);
    assert_eq!(sm.state(), M2paState::InService);
}

#[test]
fn state_machine_unexpected_transition_drops_out_of_service() {
    let mut sm = M2paStateMachine::new();
    sm.start();
    // Receiving Ready while merely NotAligned is not a valid transition.
    sm.on_link_status(LinkState::Ready);
    assert_eq!(sm.state(), M2paState::OutOfService);
}

// ── RFC 4165 2.3: what a conformant peer sends that 1.0.0 could not read ─────

#[test]
fn out_of_service_is_link_status_nine() {
    // RFC 4165 2.3.2, the table of State values: "9  Out of Service (OOS)".
    // 4.1.3: "After the association is established, M2PA SHALL send a Link
    // Status Out of Service message to its peer."
    let wire = link_status_wire(9);
    match M2paMessage::decode(&wire).unwrap() {
        M2paMessage::LinkStatus { message, .. } => {
            assert_eq!(message.state, LinkState::OutOfService);
            assert_eq!(message.state.to_string(), "Out of Service");
        }
        _ => panic!("expected LinkStatus"),
    }
    let sent = M2paMessage::LinkStatus {
        bsn: 0xFF_FFFF,
        fsn: 0xFF_FFFF,
        message: LinkStatusMessage::new(LinkState::OutOfService),
    };
    assert_eq!(sent.encode().unwrap(), wire);
}

#[test]
fn an_out_of_service_status_takes_the_state_machine_out_of_service() {
    let mut sm = M2paStateMachine::new();
    sm.start();
    sm.on_link_status(LinkState::Alignment);
    assert_eq!(sm.state(), M2paState::Aligned);
    assert_eq!(
        sm.on_link_status(LinkState::OutOfService),
        M2paState::OutOfService
    );
}

#[test]
fn a_proving_status_may_carry_filler() {
    // RFC 4165 2.3.2.1: "The Link Status Proving message may optionally carry
    // additional bytes."
    let mut wire = link_status_wire(2);
    wire.extend_from_slice(&[0xAA; 12]);
    wire[4..8].copy_from_slice(&32u32.to_be_bytes());
    match M2paMessage::decode(&wire).unwrap() {
        M2paMessage::LinkStatus { message, .. } => {
            assert_eq!(message.state, LinkState::ProvingNormal)
        }
        _ => panic!("expected LinkStatus"),
    }
}

/// A User Data message with the given BSN, FSN and Data field, written from
/// RFC 4165 2.1, 2.2 and 2.3.1.
fn user_data_wire(bsn: u32, fsn: u32, data: &[u8]) -> Vec<u8> {
    let mut v = vec![0x01, 0x00, 0x0b, 0x01];
    v.extend_from_slice(&(16 + data.len() as u32).to_be_bytes());
    v.extend_from_slice(&bsn.to_be_bytes());
    v.extend_from_slice(&fsn.to_be_bytes());
    v.extend_from_slice(data);
    v
}

#[test]
fn an_acknowledgement_only_user_data_has_no_data_field() {
    // RFC 4165 2.3.1: "The User Data is an optional field. It need not be
    // included in an acknowledgement-only message." 4.2.1: "M2PA SHOULD
    // generate and send a User Data message with no data payload".
    let wire = user_data_wire(7, 3, &[]);
    assert_eq!(wire.len(), 16);
    match M2paMessage::decode(&wire).unwrap() {
        M2paMessage::UserData { bsn, fsn, message } => {
            assert_eq!((bsn, fsn), (7, 3));
            assert!(message.is_empty());
            assert!(message.msu.is_empty());
        }
        _ => panic!("expected UserData"),
    }
    let sent = M2paMessage::UserData {
        bsn: 7,
        fsn: 3,
        message: UserDataMessage::empty(),
    };
    assert_eq!(sent.encode().unwrap(), wire);
    // A message built with no MSU is the same acknowledgement.
    let built = M2paMessage::UserData {
        bsn: 7,
        fsn: 3,
        message: UserDataMessage::new(0, Vec::new()),
    };
    assert_eq!(built.encode().unwrap(), wire);
}

#[test]
fn the_priority_is_the_two_high_bits_of_the_first_data_octet() {
    // RFC 4165 2.3.1: the first octet of the Data field is "|PRI| spare |",
    // PRI in bits 0 and 1 of a figure that numbers from the most significant bit.
    let msu = [0x83, 0x01, 0x00, 0x00, 0x00, 0x09];
    let mut data = vec![0xC0];
    data.extend_from_slice(&msu);
    let wire = user_data_wire(0, 1, &data);
    match M2paMessage::decode(&wire).unwrap() {
        M2paMessage::UserData { message, .. } => {
            assert_eq!(message.priority, 3);
            assert_eq!(message.msu, msu);
        }
        _ => panic!("expected UserData"),
    }
    let sent = M2paMessage::UserData {
        bsn: 0,
        fsn: 1,
        message: UserDataMessage::new(3, msu.to_vec()),
    };
    assert_eq!(sent.encode().unwrap(), wire);
    // The spare bits are not the priority.
    let mut spare = wire.clone();
    spare[16] = 0x03;
    match M2paMessage::decode(&spare).unwrap() {
        M2paMessage::UserData { message, .. } => assert_eq!(message.priority, 0),
        _ => panic!("expected UserData"),
    }
}
