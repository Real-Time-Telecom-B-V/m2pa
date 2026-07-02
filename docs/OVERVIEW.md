# m2pa — architecture overview

A map of the crate's internals, for contributors. (Excluded from the published
package via `Cargo.toml` `exclude`.) For usage see the [README](../README.md);
for protocol coverage see [COMPLIANCE.md](COMPLIANCE.md).

## What M2PA is (one paragraph)

M2PA (RFC 4165) is a SIGTRAN adaptation layer: it carries SS7 **MTP3** messages
over an **SCTP** association so that a signalling link that would traditionally
run over a TDM timeslot can run over IP instead. Unlike M3UA (which replaces
MTP3's routing with an application-server model), M2PA is deliberately
MTP2-shaped — it keeps BSN/FSN sequence numbers and a link that aligns, proves,
and goes in service — so MTP3 sits on top unchanged. This crate is the wire
format and that link lifecycle; it does not open sockets.

## Module map

| Path | Responsibility |
|---|---|
| `src/lib.rs` | Crate root: the `CommonMessageHeader` + `M2PAHeader` bitfields, the `M2paMessage` enum with whole-message `encode`/`decode`, protocol constants, and re-exports. |
| `src/error.rs` | `M2paError` — the typed rejection set (`thiserror`). |
| `src/link_status.rs` | `LinkState` (the 8 RFC §3.3 states) + `LinkStatusMessage` body codec. |
| `src/user_data.rs` | `UserDataMessage` — 2-bit priority + MTP3 MSU payload. |
| `src/state_machine.rs` | `M2paStateMachine` / `M2paState` — the RFC §4 link lifecycle. |
| `src/python.rs` | PyO3 bindings (`--features python`); mirrors the surface above. |

## Codec approach

The two 8-byte headers are modelled with `modular-bitfield-msb`, so the RFC's
bit layout (a 1-byte version / spare / class / type quartet + a 32-bit length; a
24-bit BSN and FSN each right-aligned in 32 bits) is expressed declaratively
rather than by hand-rolled shift-and-mask. `M2paMessage::decode` validates the
common header (version = 1, spare = 0, class = 11, type ∈ {1, 2}) before
dispatching to the body codec, and `encode` re-derives the length field so it can
never disagree with the payload.

## Public API surface (the SemVer contract)

- **Messages:** `M2paMessage` (`UserData` / `LinkStatus` variants) with
  `encode() -> Result<Vec<u8>, M2paError>` and `decode(&[u8]) -> Result<…>`.
- **Bodies:** `UserDataMessage::new/encode/decode`, `LinkStatusMessage::new/
  encode/decode`, `LinkState`.
- **Headers:** `CommonMessageHeader`, `M2PAHeader` (+ `decode_m2pa_header` /
  `encode_m2pa_header`).
- **State machine:** `M2paStateMachine` (`new`, `state`, `start`, `stop`,
  `on_link_status`), `M2paState`.
- **Constants:** `VERSION`, `MESSAGE_CLASS_M2PA`, `MESSAGE_TYPE_USER_DATA`,
  `MESSAGE_TYPE_LINK_STATUS`, `SCTP_PPID`, plus each body's `SCTP_STREAM`.

## Feature matrix

| Area | Status |
|---|---|
| Common + M2PA header pack/unpack (with validation) | ✅ |
| User Data (priority + MTP3 MSU) codec | ✅ |
| Link Status codec — all 8 states | ✅ |
| Link state machine (§4, incl. outage/recovery + busy) | ✅ |
| Python bindings (mirror + `register()` embed hook) | ✅ (`--features python`) |
| Memory: flat under encode/decode + state-machine churn | ✅ (counting-allocator leak check) |
| SCTP association / retransmission / T-timers | ⛔ out of scope (belongs to the runtime that owns the socket) |

## Where the runtime fits

A host (a SIGTRAN gateway / STP / test rig) opens the SCTP association, sends Link
Status/User Data messages produced here on the right stream (0 / 1), and pushes
each received `LinkStatusMessage.state` into an `M2paStateMachine`. When the
machine reaches `InService`, MTP3 traffic flows as `UserData`. Keeping that
socket/timer logic out of this crate is what lets the identical codec back both
the Rust crate and the Python wheel and be exhaustively unit-tested against RFC
vectors.
