# m2pa

[![crates.io](https://img.shields.io/crates/v/m2pa.svg)](https://crates.io/crates/m2pa)
[![docs.rs](https://docs.rs/m2pa/badge.svg)](https://docs.rs/m2pa)
[![CI](https://github.com/Real-Time-Telecom-B-V/m2pa/actions/workflows/ci.yml/badge.svg)](https://github.com/Real-Time-Telecom-B-V/m2pa/actions/workflows/ci.yml)
[![license](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

A pure-Rust **M2PA ([RFC 4165](https://www.rfc-editor.org/rfc/rfc4165.html))**
codec and link state machine — the **MTP2 Peer-to-Peer Adaptation Layer** that
carries SS7 MTP3 signalling over SCTP. It ships as **both** a Rust crate
(`cargo add m2pa`) and a Rust-backed Python wheel (`pip install m2pa`), built
from one source tree and one version.

M2PA lets an SS7 linkset ride IP the way it would ride a TDM signalling link:
MTP3 sits straight on top, and the link goes through the same
alignment/proving/in-service lifecycle. This crate is the **wire format**
(common header, User Data, Link Status) plus the **RFC 4165 §4 link state
machine**. It does no I/O — the SCTP association and the running link belong to
the composing runtime.

```rust
use m2pa::{LinkState, LinkStatusMessage, M2paMessage};

let msg = M2paMessage::LinkStatus {
    bsn: 0xFFFFFF,
    fsn: 0xFFFFFF,
    message: LinkStatusMessage::new(LinkState::Ready),
};
let bytes = msg.encode().unwrap();              // 20-byte M2PA message
let decoded = M2paMessage::decode(&bytes).unwrap();
```

```python
import m2pa

ls = m2pa.LinkStatus(m2pa.LinkState.Ready)      # bsn/fsn default to 0xFFFFFF
wire = ls.encode()                              # bytes
msg = m2pa.decode(wire)                          # -> LinkStatus | UserData
```

📖 More: [`docs/OVERVIEW.md`](docs/OVERVIEW.md) ·
[`docs/COMPLIANCE.md`](docs/COMPLIANCE.md) (RFC 4165 coverage) ·
[`docs/COMPARISON.md`](docs/COMPARISON.md)

## What's in the box

| Piece | Type |
|---|---|
| Common Message Header — version / spare / class / type / length | `CommonMessageHeader` |
| M2PA Header — 24-bit BSN + FSN | `M2PAHeader` |
| User Data message — priority + MTP3 MSU | `UserDataMessage` |
| Link Status message — the 8 RFC 4165 states | `LinkStatusMessage`, `LinkState` |
| Whole-message encode/decode with validation | `M2paMessage` |
| Link state machine (§4) | `M2paStateMachine`, `M2paState` |
| Typed errors | `M2paError` |

## RFC 4165 coverage

| Feature | Status |
|---|---|
| Common Message Header (v1, class 11, types User Data / Link Status) | ✅ pack/unpack + validation |
| Header validation — version = 1, spare = 0, class = 11, type ∈ {1, 2} | ✅ rejected as `M2paError` |
| M2PA Header — BSN / FSN (24-bit, right-aligned in 32) | ✅ |
| User Data message — priority (2-bit) + MTP3 MSU | ✅ `UserDataMessage` |
| Link Status message — all 8 states (Alignment … Busy Ended) | ✅ `LinkState` |
| Link state machine — OOS → Not Aligned → Aligned → Proving → Aligned Ready → In Service | ✅ `M2paStateMachine` |
| Processor outage / recovery + Busy flow-control transitions | ✅ |
| SCTP stream convention (0 = Link Status, 1 = User Data) | ✅ exposed as `SCTP_STREAM` constants |
| Registered SCTP PPID (`5`) | ✅ `SCTP_PPID` |
| SCTP association setup, retransmission/T-timers, congestion | ⛔ out of scope — belongs to the runtime that owns the socket |

Full details in [`docs/COMPLIANCE.md`](docs/COMPLIANCE.md).

## Boundary: what this crate does and doesn't do

M2PA's job splits cleanly:

- **This crate (pure, no I/O):** serialise/parse the three message shapes, and
  compute link-state transitions from received Link Status messages.
- **The composing runtime:** owns the SCTP association (multi-streaming,
  ordered/unordered delivery, the PPID), drives retransmission and the M2PA
  timers, and feeds received messages into `M2paStateMachine`. Anything that
  speaks SCTP — a SIGTRAN gateway, an STP, a test rig — can host it.

Keeping the codec I/O-free is what lets the exact same logic back the Rust crate
and the Python wheel, and makes it trivial to unit-test against RFC vectors.

## Performance

Single-core, `cargo bench` ([`benches/codec.rs`](benches/codec.rs)); the codec
is allocation-light and the state machine is branch-only. Indicative numbers:

| Operation | Time | Throughput |
|---|---|---|
| Link Status decode | ~6.5 ns | ~154 M msg/s |
| Link Status encode | ~24 ns | ~42 M msg/s |
| User Data decode (≈37-byte MSU) | ~18 ns | ~56 M msg/s |
| User Data encode | ~37 ns | ~27 M msg/s |
| Link state transition | < 1 ns | branch-only, no allocation |

A counting-allocator [leak check](examples/leak_check.rs)
(`./scripts/mem_leak_test.sh`) hammers encode/decode and the state machine and
asserts **live bytes stay flat** (Δ 0 over millions of cycles). Both run in CI.

The Python wheel is the same Rust code behind PyO3; per-call overhead is the
Python↔Rust boundary, not the codec. The module is declared `gil_used = false`,
so it loads on free-threaded ("no-GIL") CPython 3.13t / 3.14t.

## Install

```bash
cargo add m2pa          # Rust crate (zero pyo3 in the default build)
pip install m2pa        # Rust-backed Python wheel
```

## Development

```bash
cargo test                              # unit + integration + doctests
cargo test --features python            # + the PyO3 binding face
cargo clippy --all-targets -- -D warnings
cargo bench --no-run
./scripts/mem_leak_test.sh              # live-bytes leak check (PASS/FAIL)
cargo deny check                        # advisories, licenses, sources

# Python wheel
maturin develop && pytest python/tests -q
```

## License

MIT — see [LICENSE](LICENSE).
