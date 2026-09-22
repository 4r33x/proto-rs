use std::cell::Cell;
use std::cell::OnceCell;
use std::cell::UnsafeCell;
use std::mem::MaybeUninit;
use std::ptr::NonNull;
use std::sync::OnceLock;
use std::sync::atomic::AtomicU64;
use std::sync::atomic::Ordering;

use crossbeam_utils::CachePadded;

use super::RevVec;
use super::RevWriter;

/// Process-wide defaults for the one lazy encode pool on each thread.
/// Limits apply per thread, not per service or to in-flight allocations.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct EncodePoolConfig {
    /// Fixed number of leased slots per thread. Zero disables retention.
    pub max_buffers: usize,
    /// Maximum retained capacity per slot, including framing headroom.
    /// Oversized buffers are trimmed on final release. Zero disables retention.
    pub max_buffer_capacity: usize,
}

impl Default for EncodePoolConfig {
    fn default() -> Self {
        Self {
            max_buffers: 8,
            max_buffer_capacity: 32 * 1024 * 1024,
        }
    }
}

static CONFIG: OnceLock<EncodePoolConfig> = OnceLock::new();

/// Configure all thread-local encode pools once, before the first pooled encode
/// on any thread (preferably before starting runtime workers). Returns the
/// supplied configuration on error if configuration or first use already won.
/// Does not allocate payload buffers. Services need no pool handles or settings.
///
/// These limits do not restrict message size. Busy pools use temporary unpooled
/// allocations; total retained capacity multiplies by encoding-thread count.
pub fn configure_encode_pool(config: EncodePoolConfig) -> Result<(), EncodePoolConfig> {
    CONFIG.set(config)
}

thread_local! {
    static LOCAL: OnceCell<LocalPool> = const { OnceCell::new() };
}

// One ownership word per shard: the top bit belongs to TLS, the lower 63
// bits belong to active buffer leases. No separate Arc counter is necessary.
const TLS_OWNER: u64 = 1 << 63;

struct LocalPool {
    shards: Box<[ShardOwner]>,
    preferred: Cell<usize>,
}

struct Shard {
    owners: CachePadded<AtomicU64>,
    slots: Box<[UnsafeCell<MaybeUninit<RevVec>>]>,
    valid: u64,
    max_capacity: usize,
    #[cfg(test)]
    drops: std::sync::Arc<std::sync::atomic::AtomicUsize>,
}

// ShardOwner uniquely owns the TLS bit. It is deliberately neither Clone nor
// Sync: only this local handle can acquire new leases. Moving/dropping a buffer
// lease on another thread is supported independently of this handle.
struct ShardOwner(NonNull<Shard>);

struct Lease {
    shard: NonNull<Shard>,
    bit: u64,
}

// SAFETY: each lease exclusively owns one slot and one bit in the ownership
// word. All cross-thread access is confined to that slot until its Release
// publication. The bit keeps the allocation alive even after TLS destruction.
unsafe impl Send for Lease {}

impl ShardOwner {
    fn new(count: usize, max_capacity: usize) -> Self {
        debug_assert!((1..=63).contains(&count));
        let shard = Box::new(Shard {
            owners: CachePadded::new(AtomicU64::new(TLS_OWNER)),
            slots: (0..count).map(|_| UnsafeCell::new(MaybeUninit::new(RevVec::empty()))).collect(),
            valid: (1u64 << count) - 1,
            max_capacity,
            #[cfg(test)]
            drops: Default::default(),
        });
        Self(NonNull::from(Box::leak(shard)))
    }

    #[inline]
    fn take(&self) -> Option<Buffer> {
        // SAFETY: this handle still owns TLS_OWNER, keeping the shard alive.
        let shard = unsafe { self.0.as_ref() };
        let owners = shard.owners.load(Ordering::Relaxed);
        let available = !owners & shard.valid;
        if available == 0 {
            return None;
        }
        let index = available.trailing_zeros() as usize;
        let bit = 1u64 << index;
        // Only this TLS owner can set lease bits; remote returns only clear
        // them. A bit observed free stays free until we claim it, so no CAS
        // loop or spurious allocation on an unrelated return is necessary.
        shard.owners.fetch_or(bit, Ordering::Acquire);
        // SAFETY: our single-producer claim exclusively leased this slot.
        // It stays uninitialized until Buffer::drop restores it.
        let buffer = unsafe { (*shard.slots[index].get()).assume_init_read() };
        Some(Buffer {
            buffer,
            lease: Some(Lease { shard: self.0, bit }),
        })
    }
}

// SAFETY contract: the pointer is live and the caller uniquely owns 'bit'.
// Its slot (if any) must be restored before calling. No caller may access the
// shard after this RMW unless it observes itself as the final owner.
#[inline]
unsafe fn release(shard: NonNull<Shard>, bit: u64) {
    // AcqRel both publishes our slot and acquires preceding owners' writes if
    // we are last. All ownership changes are RMWs: release sequences carry
    // synchronization even when intervening operations change other bits.
    // Our bit is set and uniquely owned, so subtracting it clears only that
    // bit (no borrow). Unlike fetch_and returning the old word, fetch_sub
    // maps to one atomic subtraction on x86, not a compare-exchange retry loop.
    let previous = unsafe { shard.as_ref() }.owners.fetch_sub(bit, Ordering::AcqRel);
    debug_assert_ne!(previous & bit, 0);
    if previous == bit {
        // SAFETY: exactly one RMW can clear the final ownership bit. TLS cannot
        // acquire after releasing its bit; all leased slots are now restored.
        // Other releasers use only their saved 'previous' value after the RMW.
        unsafe { drop(Box::from_raw(shard.as_ptr())) };
    }
}

impl Drop for ShardOwner {
    fn drop(&mut self) {
        // SAFETY: this non-cloneable handle uniquely owns the TLS bit.
        unsafe { release(self.0, TLS_OWNER) };
    }
}

impl Drop for Lease {
    #[inline]
    fn drop(&mut self) {
        // SAFETY: Buffer::drop restored the slot before field destruction.
        unsafe { release(self.shard, self.bit) };
    }
}

impl Drop for Shard {
    fn drop(&mut self) {
        // SAFETY: release destroys a shard only after all ownership bits clear.
        for slot in &mut self.slots {
            unsafe { slot.get_mut().assume_init_drop() };
        }
        #[cfg(test)]
        self.drops.fetch_add(1, Ordering::Relaxed);
    }
}

impl LocalPool {
    #[cold]
    fn new() -> Self {
        Self::with_config(*CONFIG.get_or_init(EncodePoolConfig::default))
    }

    fn with_config(config: EncodePoolConfig) -> Self {
        let count = if config.max_buffer_capacity == 0 { 0 } else { config.max_buffers };
        let desired = count.div_ceil(63).max(4).min(count);
        let width = if desired == 0 { 1 } else { count.div_ceil(desired) };
        Self {
            shards: (0..count.div_ceil(width))
                .map(|group| ShardOwner::new((count - group * width).min(width), config.max_buffer_capacity))
                .collect(),
            preferred: Cell::new(0),
        }
    }

    #[inline]
    fn take(&self) -> Option<Buffer> {
        let mut group = self.preferred.get();
        for _ in 0..self.shards.len() {
            if let Some(buffer) = self.shards[group].take() {
                self.preferred.set(group);
                return Some(buffer);
            }
            group += 1;
            if group == self.shards.len() {
                group = 0;
            }
        }
        None
    }
}

pub(super) struct Buffer {
    pub(super) buffer: RevVec,
    lease: Option<Lease>,
}

impl Buffer {
    #[inline]
    pub(super) fn checkout(needed: usize) -> Self {
        LOCAL
            .try_with(|local| {
                let Some(mut owner) = local.get_or_init(LocalPool::new).take() else {
                    return Self::allocate(needed);
                };
                if owner.buffer.cap() < needed {
                    owner.reserve_empty(needed);
                }
                owner
            })
            .unwrap_or_else(|_| Self::allocate(needed))
    }

    #[cold]
    fn allocate(needed: usize) -> Self {
        Self::unpooled(RevVec::with_capacity(needed))
    }

    #[cold]
    fn reserve_empty(&mut self, needed: usize) {
        // Nothing live to copy or preserve: do not geometrically grow idle data.
        self.buffer = RevVec::with_capacity(needed);
    }

    pub(super) const fn unpooled(buffer: RevVec) -> Self {
        Self { buffer, lease: None }
    }
}

impl Drop for Buffer {
    #[inline]
    fn drop(&mut self) {
        let Some(lease) = &self.lease else { return };
        // SAFETY: our ownership bit keeps the shard alive and grants exclusive
        // access to this slot. Lease::drop publishes only after restoration.
        let shard = unsafe { lease.shard.as_ref() };
        self.buffer.clear_and_shrink(shard.max_capacity);
        let mut buffer = core::mem::replace(&mut self.buffer, RevVec::empty());
        if buffer.cap() > shard.max_capacity {
            buffer = RevVec::empty(); // A custom allocator refused to shrink.
        }
        let index = lease.bit.trailing_zeros() as usize;
        unsafe { (*shard.slots[index].get()).write(buffer) };
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::sync::Barrier;
    use std::sync::atomic::AtomicUsize;

    use super::*;

    fn pool(count: usize, capacity: usize) -> LocalPool {
        LocalPool::with_config(EncodePoolConfig {
            max_buffers: count,
            max_buffer_capacity: capacity,
        })
    }

    fn probes(pool: &LocalPool) -> Vec<Arc<AtomicUsize>> {
        // SAFETY: pool still holds all TLS ownership bits.
        pool.shards.iter().map(|s| unsafe { s.0.as_ref() }.drops.clone()).collect()
    }

    #[test]
    fn masks_cover_every_slot_exactly_once_including_owner_bit_boundaries() {
        for count in [
            0, 1, 2, 3, 4, 5, 7, 8, 62, 63, 64, 65, 125, 126, 127, 128, 129, 251, 252, 253, 255, 256, 257, 1024,
        ] {
            let pool = pool(count, 513);
            let drops = probes(&pool);
            let mut held = Vec::new();
            while let Some(buffer) = pool.take() {
                held.push(buffer);
            }
            assert_eq!(held.len(), count);
            let mut ids: Vec<_> = held
                .iter()
                .map(|b| {
                    let lease = b.lease.as_ref().unwrap();
                    assert_eq!(lease.bit & TLS_OWNER, 0);
                    (lease.shard.as_ptr().addr(), lease.bit)
                })
                .collect();
            ids.sort_unstable();
            ids.dedup();
            assert_eq!(ids.len(), count, "no duplicate live leases");
            assert!(pool.take().is_none());
            drop(held);
            for shard in &pool.shards {
                // SAFETY: each local handle still holds its ownership bit.
                assert_eq!(unsafe { shard.0.as_ref() }.owners.load(Ordering::Relaxed), TLS_OWNER);
            }
            drop(pool);
            assert!(drops.iter().all(|d| d.load(Ordering::Relaxed) == 1));
        }
        assert!(pool(8, 0).take().is_none());
    }

    #[test]
    fn final_remote_return_trims_each_allocation_and_restores_the_same_slot() {
        let pool = pool(3, 513);
        let mut held: Vec<_> = (0..3).map(|_| pool.take().unwrap()).collect();
        for (i, b) in held.iter_mut().enumerate() {
            b.buffer.put_slice(&vec![i as u8; 1024 * (i + 1)]);
        }
        assert!(pool.take().is_none());
        std::thread::spawn(move || drop(held)).join().unwrap();
        let held: Vec<_> = (0..3).map(|_| pool.take().unwrap()).collect();
        assert!(held.iter().all(|b| b.buffer.cap() == 513 && b.buffer.is_empty()));
        assert_eq!(held.iter().map(|b| b.buffer.cap()).sum::<usize>(), 3 * 513);
    }

    #[test]
    fn empty_reused_slot_reserves_the_request_without_geometric_growth() {
        std::thread::spawn(|| {
            drop(Buffer::checkout(69));
            let mut next = Buffer::checkout(513);
            assert_eq!(next.buffer.cap(), 513);
            next.buffer.put_slice(&[7; 500]);
            assert_eq!(next.buffer.as_written_slice(), &[7; 500]);
        })
        .join()
        .unwrap();
    }

    #[test]
    fn lease_survives_local_owner_and_returns_after_unwind() {
        let pool = pool(1, 69);
        let drops = probes(&pool);
        let mut b = pool.take().unwrap();
        b.buffer.put_slice(&[9; 8192]);
        drop(pool);
        assert_eq!(drops[0].load(Ordering::Relaxed), 0);
        assert_eq!(b.buffer.as_written_slice(), &[9; 8192]);
        drop(b);
        assert_eq!(drops[0].load(Ordering::Relaxed), 1);
        let pool = self::pool(1, 69);
        assert!(
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                let mut b = pool.take().unwrap();
                b.buffer.put_slice(&[7; 8192]);
                panic!("injected encoding panic");
            }))
            .is_err()
        );
        let b = pool.take().unwrap();
        assert_eq!(b.buffer.cap(), 69);
        assert!(b.buffer.is_empty());
    }

    #[test]
    fn tls_exhaustion_does_not_evict_and_leases_survive_thread_exit() {
        let (held, drops) = std::thread::spawn(|| {
            let count = CONFIG.get_or_init(EncodePoolConfig::default).max_buffers;
            let mut held: Vec<_> = (0..count).map(|_| Buffer::checkout(128)).collect();
            assert!(held.iter().all(|b| b.lease.is_some()));
            for (i, b) in held.iter_mut().enumerate() {
                b.buffer.put_slice(&[i as u8; 100]);
            }
            assert!(Buffer::checkout(128).lease.is_none());
            let drops = LOCAL.with(|l| probes(l.get().unwrap()));
            (held, drops)
        })
        .join()
        .unwrap();
        assert!(drops.iter().all(|d| d.load(Ordering::Relaxed) == 0));
        for (i, b) in held.iter().enumerate() {
            assert_eq!(b.buffer.as_written_slice(), &[i as u8; 100]);
        }
        drop(held);
        assert!(drops.iter().all(|d| d.load(Ordering::Relaxed) == 1));
    }

    #[test]
    fn concurrent_returns_race_local_owner_destruction_without_double_free() {
        for count in [1, 2, 8, 63, 64, 127, 253] {
            for _ in 0..8 {
                let pool = pool(count, 127);
                let drops = probes(&pool);
                let mut batches: [Vec<Buffer>; 4] = std::array::from_fn(|_| Vec::new());
                for i in 0..count {
                    let mut b = pool.take().unwrap();
                    b.buffer.put_slice(&[42; 1024]);
                    batches[i % 4].push(b);
                }
                let barrier = &Barrier::new(5);
                std::thread::scope(|s| {
                    for batch in batches {
                        s.spawn(move || {
                            barrier.wait();
                            for b in batch {
                                assert_eq!(b.buffer.as_written_slice(), &[42; 1024]);
                                drop(b);
                                std::thread::yield_now();
                            }
                        });
                    }
                    barrier.wait();
                    drop(pool);
                });
                assert!(drops.iter().all(|d| d.load(Ordering::Relaxed) == 1));
            }
        }
    }

    #[test]
    fn producer_claims_slots_while_multiple_consumers_return_them() {
        let pool = pool(65, 127);
        let drops = probes(&pool);
        std::thread::scope(|scope| {
            let senders: Vec<_> = (0..4)
                .map(|_| {
                    let (tx, rx) = std::sync::mpsc::sync_channel::<(Buffer, u8)>(8);
                    scope.spawn(move || {
                        for (b, value) in rx {
                            std::thread::yield_now();
                            assert_eq!(b.buffer.as_written_slice(), &[value; 512]);
                            drop(b);
                        }
                    });
                    tx
                })
                .collect();
            for i in 0..512 {
                let mut b = pool.take().unwrap_or_else(|| Buffer::unpooled(RevVec::empty()));
                b.buffer.put_slice(&[i as u8; 512]);
                senders[i % 4].send((b, i as u8)).unwrap();
            }
            drop(pool); // queued leases can still be owned by the consumers
            drop(senders);
        });
        assert!(drops.iter().all(|d| d.load(Ordering::Relaxed) == 1));
    }
}
