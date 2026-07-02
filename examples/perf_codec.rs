//! Codec throughput driver — a runnable "how fast is it" companion to the
//! criterion benches. Encodes and decodes millions of messages in a tight loop
//! and prints sustained messages/second for each shape.
//!
//! Run: `cargo run --release --example perf_codec`
//!      `ITERS=50000000 cargo run --release --example perf_codec`
//!
//! This measures the pure codec (no I/O, single thread). The state machine is
//! branch-only and effectively free, so it is reported separately.

use std::time::Instant;

use m2pa::{LinkState, LinkStatusMessage, M2paMessage, M2paStateMachine, UserDataMessage};

fn iters() -> u64 {
    std::env::var("ITERS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(20_000_000)
}

fn rate(label: &str, n: u64, secs: f64) {
    let per_s = n as f64 / secs;
    println!(
        "  {label:<28} {n} in {secs:.3}s  =>  {:>7.1} M/s  ({:.1} ns each)",
        per_s / 1e6,
        secs * 1e9 / n as f64
    );
}

fn main() {
    let n = iters();
    println!("[perf] {n} iterations each (release, single thread)\n");

    // Fixtures.
    let mut msu = vec![0x83, 0x01, 0x02, 0x03, 0x04];
    msu.extend_from_slice(&[0xAB; 32]);
    let link_status = M2paMessage::LinkStatus {
        bsn: 0x00FF_FFFF,
        fsn: 0x00FF_FFFF,
        message: LinkStatusMessage::new(LinkState::Ready),
    };
    let user_data = M2paMessage::UserData {
        bsn: 0x0012_3456,
        fsn: 0x0012_3457,
        message: UserDataMessage::new(1, msu),
    };
    let ls_bytes = link_status.encode().unwrap();
    let ud_bytes = user_data.encode().unwrap();

    // Link Status encode.
    let t = Instant::now();
    for _ in 0..n {
        std::hint::black_box(link_status.encode().unwrap());
    }
    rate("link_status/encode", n, t.elapsed().as_secs_f64());

    // Link Status decode.
    let t = Instant::now();
    for _ in 0..n {
        std::hint::black_box(M2paMessage::decode(&ls_bytes).unwrap());
    }
    rate("link_status/decode", n, t.elapsed().as_secs_f64());

    // User Data encode.
    let t = Instant::now();
    for _ in 0..n {
        std::hint::black_box(user_data.encode().unwrap());
    }
    rate("user_data/encode", n, t.elapsed().as_secs_f64());

    // User Data decode.
    let t = Instant::now();
    for _ in 0..n {
        std::hint::black_box(M2paMessage::decode(&ud_bytes).unwrap());
    }
    rate("user_data/decode", n, t.elapsed().as_secs_f64());

    // State machine: full alignment sequences.
    let t = Instant::now();
    for _ in 0..n {
        let mut sm = M2paStateMachine::new();
        sm.start();
        sm.on_link_status(LinkState::Alignment);
        sm.on_link_status(LinkState::ProvingNormal);
        sm.on_link_status(LinkState::Ready);
        std::hint::black_box(sm.on_link_status(LinkState::Ready));
    }
    rate("state_machine/alignment", n, t.elapsed().as_secs_f64());
}
