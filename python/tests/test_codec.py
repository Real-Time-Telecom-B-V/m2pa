"""Codec parity / round-trip tests for the m2pa wheel.

These exercise the same Rust codec and state machine the crate ships, through the
Python surface: ``encode`` must match the RFC 4165 wire form, ``decode`` must
recover the fields, and the state machine must follow §4.
"""

from __future__ import annotations

import pytest

import m2pa

# RFC 4165 wire form of a Link Status "Ready" (state = 4): common header
# 01 00 0b 02 + length 0x14 + BSN/FSN 00ffffff each + state 00000004.
GOLDEN_LINK_STATUS_READY = bytes.fromhex(
    "01000b020000001400ffffff00ffffff00000004"
)

# pyo3 enums aren't Python IntEnums (not iterable), so list the states explicitly.
ALL_LINK_STATES = [
    m2pa.LinkState.Alignment,
    m2pa.LinkState.ProvingNormal,
    m2pa.LinkState.ProvingEmergency,
    m2pa.LinkState.Ready,
    m2pa.LinkState.ProcessorOutage,
    m2pa.LinkState.ProcessorRecovered,
    m2pa.LinkState.Busy,
    m2pa.LinkState.BusyEnded,
]


def test_constants() -> None:
    assert m2pa.VERSION == 1
    assert m2pa.MESSAGE_CLASS == 11
    assert m2pa.MESSAGE_TYPE_USER_DATA == 1
    assert m2pa.MESSAGE_TYPE_LINK_STATUS == 2
    assert m2pa.SCTP_PPID == 5


def test_link_status_matches_golden_vector() -> None:
    ls = m2pa.LinkStatus(m2pa.LinkState.Ready)  # bsn/fsn default to 0xFFFFFF
    assert ls.encode() == GOLDEN_LINK_STATUS_READY


def test_link_state_wire_values() -> None:
    # The enum's integer value is the on-wire encoding.
    assert int(m2pa.LinkState.Alignment) == 1
    assert int(m2pa.LinkState.ProvingEmergency) == 3
    assert int(m2pa.LinkState.BusyEnded) == 8


def test_decode_golden_link_status() -> None:
    msg = m2pa.decode(GOLDEN_LINK_STATUS_READY)
    assert isinstance(msg, m2pa.LinkStatus)
    assert msg.state == m2pa.LinkState.Ready
    assert msg.bsn == 0xFFFFFF
    assert msg.fsn == 0xFFFFFF


@pytest.mark.parametrize("state", ALL_LINK_STATES)
def test_link_status_round_trip_all_states(state) -> None:
    wire = m2pa.LinkStatus(state, bsn=1, fsn=2).encode()
    decoded = m2pa.decode(wire)
    assert isinstance(decoded, m2pa.LinkStatus)
    assert decoded.state == state
    assert decoded.bsn == 1
    assert decoded.fsn == 2
    # re-encoding reproduces the exact bytes
    assert decoded.encode() == wire


def test_user_data_round_trip() -> None:
    msu = bytes([0x83, 0x01, 0x02, 0x03]) + bytes(range(64))
    ud = m2pa.UserData(msu, priority=2, bsn=100, fsn=101)
    wire = ud.encode()
    decoded = m2pa.decode(wire)
    assert isinstance(decoded, m2pa.UserData)
    assert decoded.bsn == 100
    assert decoded.fsn == 101
    assert decoded.priority == 2
    assert decoded.msu == msu
    assert decoded.encode() == wire


def test_user_data_priority_masked_to_two_bits() -> None:
    ud = m2pa.UserData(b"\x01", priority=0xFF)
    assert ud.priority == 3  # 0xFF & 0x03


def test_user_data_empty_msu() -> None:
    ud = m2pa.UserData(b"")
    assert ud.msu == b""
    assert m2pa.decode(ud.encode()).msu == b""


def test_decode_rejects_unknown_state() -> None:
    bad = bytes.fromhex("01000b020000001400ffffff00ffffff00000009")  # state 9
    with pytest.raises(m2pa.M2paError):
        m2pa.decode(bad)


def test_decode_rejects_truncated() -> None:
    with pytest.raises(m2pa.M2paError):
        m2pa.decode(b"\x01\x00\x0b\x02")


def test_decode_rejects_bad_version() -> None:
    bad = bytearray(GOLDEN_LINK_STATUS_READY)
    bad[0] = 9
    with pytest.raises(m2pa.M2paError):
        m2pa.decode(bytes(bad))


def test_state_machine_full_alignment() -> None:
    sm = m2pa.StateMachine()
    assert sm.state == m2pa.M2paState.OutOfService
    assert sm.start() is True
    assert sm.state == m2pa.M2paState.NotAligned
    sm.on_link_status(m2pa.LinkState.Alignment)
    assert sm.state == m2pa.M2paState.Aligned
    sm.on_link_status(m2pa.LinkState.ProvingNormal)
    assert sm.state == m2pa.M2paState.Proving
    sm.on_link_status(m2pa.LinkState.Ready)
    assert sm.state == m2pa.M2paState.AlignedReady
    result = sm.on_link_status(m2pa.LinkState.Ready)
    assert result == m2pa.M2paState.InService
    assert sm.state == m2pa.M2paState.InService


def test_state_machine_outage_recovery() -> None:
    sm = m2pa.StateMachine()
    sm.start()
    for ls in (
        m2pa.LinkState.Alignment,
        m2pa.LinkState.ProvingNormal,
        m2pa.LinkState.Ready,
        m2pa.LinkState.Ready,
    ):
        sm.on_link_status(ls)
    assert sm.state == m2pa.M2paState.InService
    sm.on_link_status(m2pa.LinkState.ProcessorOutage)
    assert sm.state == m2pa.M2paState.AlignedReady
    sm.on_link_status(m2pa.LinkState.ProcessorRecovered)
    assert sm.state == m2pa.M2paState.InService
    sm.stop()
    assert sm.state == m2pa.M2paState.OutOfService
