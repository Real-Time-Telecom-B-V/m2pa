# Comparison

Where `m2pa` sits relative to other ways of speaking M2PA. This is a **focused
codec + link state machine**, not a full SS7 stack — so the honest comparison is
about scope and integration cost, not feature count.

| | m2pa | Full C SS7 stacks (Dialogic, ss7 suites) | Java telecom frameworks (jSS7 / RestComm lineage) | Wireshark M2PA dissector |
|---|---|---|---|---|
| Language | Rust (+ Python wheel) | C | Java | C (decode only) |
| Scope | M2PA message format + §4 link state machine | Whole SS7 stack (MTP2/3, SCCP, ISUP, TCAP, …) | Whole stack incl. M3UA/SCCP/TCAP/MAP | Packet dissection only |
| I/O model | none — you bring the SCTP association | integrated transport + timers | integrated transport + timers | n/a (reads captures) |
| Encode + decode | ✅ both | ✅ | ✅ | decode only |
| License | MIT | commercial / mixed | AGPL/commercial (varies) | GPL-2.0 |
| Embed cost | `cargo add` / `pip install`, no runtime | heavy; platform + licensing | JVM + framework | not a library |
| Footprint | two small deps, alloc-light | large | large (JVM) | — |

## When m2pa is the right tool

- You already have (or are writing) an **SCTP runtime** — a SIGTRAN gateway, an
  STP, a test harness — and you want a correct, tested M2PA **message layer**
  plus the link lifecycle, without adopting a whole stack or a C/JVM dependency.
- You want the **same** codec in a Rust service and in Python tooling (test
  rigs, packet builders, lab automation) from one source of truth.
- You value an MIT, dependency-light, RFC-vector-tested building block over a
  turnkey but heavyweight framework.

## When it is not

- You need a **turnkey SS7 stack** (MTP3 routing, SCCP, TCAP, MAP/CAP, ISUP) out
  of the box — reach for a full stack; `m2pa` is one layer of that picture.
- You need the **SCTP transport, retransmission, and M2PA timers** bundled in —
  those live in the runtime that hosts the link, not here (see
  [COMPLIANCE.md](COMPLIANCE.md) for the scope boundary).

## Design stance

M2PA's value is precisely that it is thin: MTP2-shaped framing over SCTP so MTP3
rides unchanged. Modelling just that — the three message shapes and the
alignment/proving/in-service machine — as a pure, I/O-free library keeps it easy
to test, easy to embed, and identical across Rust and Python.
