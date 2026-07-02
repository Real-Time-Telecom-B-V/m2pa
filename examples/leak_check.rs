//! Memory-leak check.
//!
//! A counting global allocator tracks **live bytes** (allocated − freed) — RSS
//! is too noisy (the OS/allocator retains freed pages), but live bytes are
//! exact, so a real leak shows up as monotonic growth. Two phases:
//!
//!   1. **codec** — encode + decode a Link Status and a User Data message for
//!      many cycles (the header pack/unpack + body copy path).
//!   2. **state machine** — drive a fresh state machine through a full alignment
//!      + processor-outage recovery, over and over.
//!
//! Each phase asserts live bytes return to a flat baseline. Exits non-zero on a
//! leak. Driven by `scripts/mem_leak_test.sh`.
//!
//! Run: `cargo run --release --example leak_check`

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicI64, Ordering};

use m2pa::{LinkState, LinkStatusMessage, M2paMessage, M2paStateMachine, UserDataMessage};

// ── Counting allocator ──────────────────────────────────────────────────────
static LIVE: AtomicI64 = AtomicI64::new(0);

struct Counting;
unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, l: Layout) -> *mut u8 {
        let p = System.alloc(l);
        if !p.is_null() {
            LIVE.fetch_add(l.size() as i64, Ordering::Relaxed);
        }
        p
    }
    unsafe fn dealloc(&self, p: *mut u8, l: Layout) {
        System.dealloc(p, l);
        LIVE.fetch_sub(l.size() as i64, Ordering::Relaxed);
    }
    unsafe fn alloc_zeroed(&self, l: Layout) -> *mut u8 {
        let p = System.alloc_zeroed(l);
        if !p.is_null() {
            LIVE.fetch_add(l.size() as i64, Ordering::Relaxed);
        }
        p
    }
    unsafe fn realloc(&self, ptr: *mut u8, l: Layout, new_size: usize) -> *mut u8 {
        let p = System.realloc(ptr, l, new_size);
        if !p.is_null() {
            LIVE.fetch_add(new_size as i64 - l.size() as i64, Ordering::Relaxed);
        }
        p
    }
}

#[global_allocator]
static ALLOC: Counting = Counting;

fn live() -> i64 {
    LIVE.load(Ordering::Relaxed)
}

// ── Phase 1: codec workload ─────────────────────────────────────────────────
fn codec_cycle(iters: usize) {
    let link_status = M2paMessage::LinkStatus {
        bsn: 0x00FF_FFFF,
        fsn: 0x00FF_FFFF,
        message: LinkStatusMessage::new(LinkState::Ready),
    };
    let mut msu = vec![0x83, 0x01, 0x02, 0x03, 0x04];
    msu.extend_from_slice(&[0xAB; 32]);
    let user_data = M2paMessage::UserData {
        bsn: 0x0012_3456,
        fsn: 0x0012_3457,
        message: UserDataMessage::new(1, msu),
    };
    for _ in 0..iters {
        let ls = link_status.encode().unwrap();
        std::hint::black_box(M2paMessage::decode(&ls).unwrap());
        let ud = user_data.encode().unwrap();
        std::hint::black_box(M2paMessage::decode(&ud).unwrap());
    }
}

// ── Phase 2: state-machine churn ────────────────────────────────────────────
fn state_machine_cycle(iters: usize) {
    for _ in 0..iters {
        let mut sm = M2paStateMachine::new();
        sm.start();
        sm.on_link_status(LinkState::Alignment);
        sm.on_link_status(LinkState::ProvingNormal);
        sm.on_link_status(LinkState::Ready);
        sm.on_link_status(LinkState::Ready);
        sm.on_link_status(LinkState::ProcessorOutage);
        sm.on_link_status(LinkState::ProcessorRecovered);
        std::hint::black_box(sm.state());
    }
}

fn report(phase: &str, base: i64) -> i64 {
    let growth = live() - base;
    println!("  {phase}: live = {} bytes (Δ {:+})", live(), growth);
    growth
}

fn main() {
    const ITERS: usize = 200_000;
    const CYCLES: usize = 10;
    const BUDGET: i64 = 64 * 1024;

    // Phase 1: codec.
    println!("[codec] {CYCLES} x {ITERS} encode+decode round-trips (link status + user data)");
    codec_cycle(ITERS); // warm up
    let codec_base = live();
    for c in 1..=CYCLES {
        codec_cycle(ITERS);
        report(&format!("cycle {c:>2}/{CYCLES}"), codec_base);
    }
    let codec_growth = live() - codec_base;

    // Phase 2: state machine.
    println!("\n[state machine] {CYCLES} x {ITERS} full alignment + outage recovery");
    state_machine_cycle(ITERS); // warm up
    let sm_base = live();
    for c in 1..=CYCLES {
        state_machine_cycle(ITERS);
        report(&format!("cycle {c:>2}/{CYCLES}"), sm_base);
    }
    let sm_growth = live() - sm_base;

    // Verdict.
    println!();
    let mut ok = true;
    if codec_growth > BUDGET {
        eprintln!("FAIL: codec live bytes grew {codec_growth} (> {BUDGET})");
        ok = false;
    }
    if sm_growth > BUDGET {
        eprintln!("FAIL: state-machine live bytes grew {sm_growth} (> {BUDGET})");
        ok = false;
    }
    if !ok {
        std::process::exit(1);
    }
    println!("PASS: codec Δ {codec_growth} ≤ {BUDGET}; state-machine Δ {sm_growth} ≤ {BUDGET}");
}
