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
    // State 9 is not defined in RFC 4165 §3.3.
    let err = M2paMessage::decode(&link_status_wire(9)).unwrap_err();
    assert!(matches!(err, M2paError::InvalidLinkStatus(9)));
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
