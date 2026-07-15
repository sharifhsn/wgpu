use core::{
    cell::UnsafeCell,
    fmt::{self, Write as _},
    sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering},
};

const TRACE_CAPACITY: usize = 256;
const EVENT_CAPACITY: usize = 768;

struct TraceRing {
    locked: AtomicBool,
    next: AtomicUsize,
    count: AtomicUsize,
    lengths: UnsafeCell<[u16; TRACE_CAPACITY]>,
    events: UnsafeCell<[[u8; EVENT_CAPACITY]; TRACE_CAPACITY]>,
}

unsafe impl Sync for TraceRing {}

impl TraceRing {
    const fn new() -> Self {
        Self {
            locked: AtomicBool::new(false),
            next: AtomicUsize::new(0),
            count: AtomicUsize::new(0),
            lengths: UnsafeCell::new([0; TRACE_CAPACITY]),
            events: UnsafeCell::new([[0; EVENT_CAPACITY]; TRACE_CAPACITY]),
        }
    }

    fn lock(&self) {
        while self
            .locked
            .compare_exchange_weak(false, true, Ordering::Acquire, Ordering::Relaxed)
            .is_err()
        {
            core::hint::spin_loop();
        }
    }

    fn unlock(&self) {
        self.locked.store(false, Ordering::Release);
    }
}

struct EventWriter {
    bytes: [u8; EVENT_CAPACITY],
    len: usize,
}

impl fmt::Write for EventWriter {
    fn write_str(&mut self, value: &str) -> fmt::Result {
        let remaining = EVENT_CAPACITY.saturating_sub(self.len);
        let length = value.len().min(remaining);
        self.bytes[self.len..self.len + length].copy_from_slice(&value.as_bytes()[..length]);
        self.len += length;
        Ok(())
    }
}

static NEXT_RESOURCE_ID: AtomicU64 = AtomicU64::new(1);
static RESOURCE_EVENTS: TraceRing = TraceRing::new();
static COMMAND_EVENTS: TraceRing = TraceRing::new();

pub(super) fn resource_id() -> u64 {
    NEXT_RESOURCE_ID.fetch_add(1, Ordering::Relaxed)
}

fn record_in(ring: &TraceRing, args: fmt::Arguments<'_>, rolling: bool) {
    let mut writer = EventWriter {
        bytes: [0; EVENT_CAPACITY],
        len: 0,
    };
    let _ = writer.write_fmt(args);
    ring.lock();
    let count = ring.count.load(Ordering::Relaxed);
    if !rolling && count == TRACE_CAPACITY {
        ring.unlock();
        return;
    }
    let index = ring.next.load(Ordering::Relaxed);
    unsafe {
        (&mut (*ring.events.get())[index])[..writer.len]
            .copy_from_slice(&writer.bytes[..writer.len]);
        (*ring.lengths.get())[index] = writer.len as u16;
    }
    ring.next
        .store((index + 1) % TRACE_CAPACITY, Ordering::Relaxed);
    ring.count
        .store((count + 1).min(TRACE_CAPACITY), Ordering::Relaxed);
    ring.unlock();
}

pub(super) fn record(args: fmt::Arguments<'_>) {
    record_in(&COMMAND_EVENTS, args, true);
}

pub(super) fn record_resource(args: fmt::Arguments<'_>) {
    record_in(&RESOURCE_EVENTS, args, false);
}

fn dump_ring(ring: &TraceRing) {
    let count = ring.count.load(Ordering::Relaxed);
    let next = ring.next.load(Ordering::Relaxed);
    for offset in 0..count {
        let index = (next + TRACE_CAPACITY - count + offset) % TRACE_CAPACITY;
        let length = unsafe { (*ring.lengths.get())[index] as usize };
        let bytes = unsafe { &(&(*ring.events.get())[index])[..length] };
        let event = core::str::from_utf8(bytes).unwrap_or("<invalid trace event>");
        eprintln!("[wgpu-deko3d-trace] {event}");
    }
}

pub(super) fn dump(reason: &str) {
    RESOURCE_EVENTS.lock();
    COMMAND_EVENTS.lock();
    let count = RESOURCE_EVENTS.count.load(Ordering::Relaxed)
        + COMMAND_EVENTS.count.load(Ordering::Relaxed);
    eprintln!("[wgpu-deko3d-trace] begin reason={reason} events={count}");
    dump_ring(&RESOURCE_EVENTS);
    dump_ring(&COMMAND_EVENTS);
    eprintln!("[wgpu-deko3d-trace] end");
    COMMAND_EVENTS.unlock();
    RESOURCE_EVENTS.unlock();
}

pub(super) fn fingerprint(bytes: &[u8]) -> u64 {
    let mut hash = 0xcbf29ce484222325_u64;
    for &byte in bytes {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}
