//! Codec micro-benchmarks: M2PA message encode/decode + state-machine stepping.
//!
//! Run with `cargo bench`. Numbers feed the README "Performance" table.
//!
//! All fixtures are built from the public API (or the RFC wire layout), so the
//! benches measure exactly the work this crate does — header pack/unpack, body
//! copy, and the state transition table — with no I/O in the path.

use criterion::{criterion_group, criterion_main, BatchSize, Criterion, Throughput};
use m2pa::{LinkState, LinkStatusMessage, M2paMessage, M2paStateMachine, UserDataMessage};

/// A representative MTP3 MSU payload (synthetic: SIO + routing label + a short
/// SCCP-ish body). Length is what matters for the copy path, not the contents.
fn sample_msu() -> Vec<u8> {
    let mut msu = vec![0x83, 0x01, 0x02, 0x03, 0x04];
    msu.extend_from_slice(&[0x09, 0x00, 0x03, 0x05, 0x0a, 0x0b, 0x0c, 0x0d]);
    msu.extend_from_slice(&[0xAB; 24]);
    msu
}

fn bench_codec(c: &mut Criterion) {
    // Fixtures.
    let link_status = M2paMessage::LinkStatus {
        bsn: 0x00FF_FFFF,
        fsn: 0x00FF_FFFF,
        message: LinkStatusMessage::new(LinkState::Ready),
    };
    let user_data = M2paMessage::UserData {
        bsn: 0x0012_3456,
        fsn: 0x0012_3457,
        message: UserDataMessage::new(1, sample_msu()),
    };
    let link_status_bytes = link_status.encode().expect("valid link status");
    let user_data_bytes = user_data.encode().expect("valid user data");

    let mut g = c.benchmark_group("codec");
    g.throughput(Throughput::Elements(1));

    g.bench_function("link_status/decode", |b| {
        b.iter(|| M2paMessage::decode(&link_status_bytes).unwrap())
    });
    g.bench_function("link_status/encode", |b| {
        b.iter_batched(
            || link_status.clone(),
            |m| m.encode().unwrap(),
            BatchSize::SmallInput,
        )
    });
    g.bench_function("user_data/decode", |b| {
        b.iter(|| M2paMessage::decode(&user_data_bytes).unwrap())
    });
    g.bench_function("user_data/encode", |b| {
        b.iter_batched(
            || user_data.clone(),
            |m| m.encode().unwrap(),
            BatchSize::SmallInput,
        )
    });
    g.finish();

    // The link state machine: a full alignment sequence (5 transitions).
    let mut sg = c.benchmark_group("state_machine");
    sg.throughput(Throughput::Elements(1));
    sg.bench_function("full_alignment", |b| {
        b.iter(|| {
            let mut sm = M2paStateMachine::new();
            sm.start();
            sm.on_link_status(LinkState::Alignment);
            sm.on_link_status(LinkState::ProvingNormal);
            sm.on_link_status(LinkState::Ready);
            sm.on_link_status(LinkState::Ready)
        })
    });
    sg.finish();
}

criterion_group!(benches, bench_codec);
criterion_main!(benches);
