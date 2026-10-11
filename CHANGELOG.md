# Changelog

All notable changes to this project are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and the project adheres
to [Semantic Versioning](https://semver.org/spec/v2.0.0.html). See
[VERSIONING.md](VERSIONING.md) for the compatibility policy.

## [1.1.0]

### Added
- **`LinkState::OutOfService`** (value 9, RFC 4165 2.3.2). 1.0.0 had eight of
  the nine states: the Link Status Out of Service a peer sends once the
  association is up (4.1.3, "SHALL") did not decode, and could not be sent. An
  exhaustive `match` on `LinkState` needs the new arm.
- **`UserDataMessage::empty()` / `is_empty()`**: the acknowledgement-only User
  Data message, which has no Data field (2.3.1, 4.2.1).

### Fixed
- **A User Data message without a Data field was a decode error**, so the
  acknowledgement a peer sends when it has nothing else to send was rejected. It
  decodes as an empty message, and a message built with no MSU encodes to the
  16 octets of the two headers and nothing after them (1.0.0 wrote a lone
  priority octet).
- **The priority was read from and written to the two low bits of the first
  Data octet.** RFC 4165 2.3.1 has it in the two most significant bits, the low
  six being spare. Only the national MTP that uses this field (priority other
  than 0) saw the difference on the wire.

## [1.0.0]

First published release — the M2PA (RFC 4165) codec + link state machine, shipped
as both a Rust crate (crates.io) and a Rust-backed Python wheel (PyPI) from one
source tree.

### Added
- **Codec** — `M2paMessage` whole-message encode/decode with header validation
  (version = 1, spare = 0, class = 11, type ∈ {User Data, Link Status}).
  - `CommonMessageHeader` (version / spare / class / type / length) and
    `M2PAHeader` (24-bit BSN + FSN), modelled with MSB-first bitfields.
  - `UserDataMessage` — 2-bit priority + MTP3 MSU payload (SCTP stream 1).
  - `LinkStatusMessage` + `LinkState` — all 8 RFC 4165 §3.3 states (SCTP stream 0).
- **Link state machine** — `M2paStateMachine` / `M2paState`: the RFC 4165 §4
  lifecycle (Out of Service → Not Aligned → Aligned → Proving → Aligned Ready →
  In Service), plus processor outage/recovery and Busy flow-control transitions.
- **Typed errors** — `M2paError` (`thiserror`) for every rejection.
- **Python bindings** (`pip install m2pa`, feature `python`) — `LinkStatus`,
  `UserData`, `StateMachine`, `LinkState`, `M2paState`, `decode()`, and the
  protocol constants. Declared `gil_used = false` for free-threaded CPython.
  A `register(py, parent)` entry point mounts `m2pa` as a submodule of a host
  extension.
- **Quality bar** — criterion benches (`benches/codec.rs`), a counting-allocator
  leak check (`examples/leak_check.rs` + `scripts/mem_leak_test.sh`), a codec
  throughput driver (`examples/perf_codec.rs`), pytest parity tests, RFC-derived
  integration vectors, and CI (fmt / clippy / test / bench-compile / leak gate /
  wheel + pytest / cargo-deny).

[1.1.0]: https://github.com/Real-Time-Telecom-B-V/m2pa/releases/tag/v1.1.0
[1.0.0]: https://github.com/Real-Time-Telecom-B-V/m2pa/releases/tag/v1.0.0
