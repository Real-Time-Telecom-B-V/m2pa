# RFC 4165 compliance

What of [RFC 4165](https://www.rfc-editor.org/rfc/rfc4165.html) ("SS7 MTP2-User
Peer-to-Peer Adaptation Layer") `m2pa` implements, section by section. The scope
is the **message format** and the **link state machine**; the SCTP association
and the M2PA timers belong to the runtime that hosts the link (see
[OVERVIEW.md](OVERVIEW.md)).

## Message format

| RFC | Item | Status | Notes |
|---|---|---|---|
| §2.1 | Common Message Header — Version, Spare, Message Class, Message Type, Message Length | ✅ | `CommonMessageHeader`; validated on decode |
| §2.1 | Version = 1 | ✅ | non-`1` → `M2paError::InvalidVersion` |
| §2.1 | Spare = 0 | ✅ | non-`0` → `M2paError::InvalidSpare` |
| §2.1 | Message Class = 11 (M2PA) | ✅ | else `M2paError::InvalidMessageClass` |
| §2.1 | Message Type — 1 = User Data, 2 = Link Status | ✅ | else `M2paError::InvalidMessageType` |
| §2.2 | M2PA-specific header — BSN, FSN (24-bit, right-aligned in 32 bits, top octet unused) | ✅ | `M2PAHeader` |
| §3.2 | User Data message — Priority + User Data (MTP3 MSU) | ✅ | `UserDataMessage`; priority masked to its 2 valid bits |
| §3.3 | Link Status message — State field | ✅ | `LinkStatusMessage` |
| §3.3 | Link Status states 1–8: Alignment, Proving Normal, Proving Emergency, Ready, Processor Outage, Processor Recovered, Busy, Busy Ended | ✅ | `LinkState`; unknown value → `M2paError::InvalidLinkStatus` |

## Link state machine (§4)

| Transition | Status |
|---|---|
| Out of Service → Not Aligned (`start()`) | ✅ |
| Not Aligned → Aligned (peer Alignment) | ✅ |
| Aligned → Proving (peer Proving Normal / Emergency) | ✅ |
| Proving → Aligned Ready (peer Ready) | ✅ |
| Aligned Ready → In Service (peer Ready) | ✅ |
| In Service → Aligned Ready (Processor Outage) | ✅ |
| Aligned Ready → In Service (Processor Recovered) | ✅ |
| In Service holds through Busy / Busy Ended (flow control) | ✅ |
| Any unexpected input → Out of Service (fail safe) | ✅ |

## SCTP considerations (RFC 4165 §1.4, §5)

| Item | Status | Notes |
|---|---|---|
| Registered SCTP PPID = 5 | ✅ constant | `SCTP_PPID` (the runtime sets it on the association) |
| Stream usage — Link Status on stream 0, User Data on stream 1 | ✅ constants | `LinkStatusMessage::SCTP_STREAM` / `UserDataMessage::SCTP_STREAM` |
| SCTP association establishment / teardown | ⛔ | out of scope — runtime owns the socket |
| Retransmission, T1–T7 timers, congestion / flow control beyond Busy signalling | ⛔ | out of scope — runtime concern |

## Deliberate scope boundary

`m2pa` is an I/O-free codec + state machine, by design: that is what makes it
unit-testable against RFC-derived wire vectors, embeddable in any SCTP runtime,
and shareable byte-for-byte between the Rust crate and the Python wheel. A
compliant M2PA *link* additionally needs the SCTP transport and the M2PA timers,
which the composing runtime supplies.

## Test vectors

Every vector in `tests/integration.rs` (and the unit tests) is built from the
spec, not captured traffic. Link Status messages are pure control signalling
(header + BSN/FSN + a state word) with no subscriber or routing data; User Data
tests use synthetic MTP3 payloads. Where a hex string is pinned, a paired encode
test proves the crate reproduces those exact bytes.
