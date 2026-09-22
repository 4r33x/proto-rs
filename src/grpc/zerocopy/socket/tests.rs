use super::*;

#[test]
fn channel_metrics_retire_closed_connections_without_losing_totals() {
    let channel = crate::grpc::zerocopy::ZeroCopyChannelMetrics::default();
    let mut weak = Vec::new();
    for _ in 0..16 {
        let metrics = ZeroCopyMetrics::default();
        metrics.0.closed.store(true, Ordering::Release);
        metrics.0.send_calls.store(1, Ordering::Relaxed);
        metrics.0.submitted.store(1024, Ordering::Relaxed);
        metrics.0.copied.store(1024, Ordering::Relaxed);
        weak.push(Arc::downgrade(&metrics.0));
        channel.register(metrics);
    }
    let snapshot = channel.snapshot();
    assert_eq!(channel.connections(), 16);
    assert_eq!(snapshot.send_calls, 16);
    assert_eq!(snapshot.submitted_bytes, 16384);
    assert_eq!(snapshot.completed_with_copy_flag, 16384);
    assert!(
        weak.into_iter().all(|entry| entry.upgrade().is_none()),
        "retired metrics must not accumulate"
    );
}

#[test]
fn channel_metrics_keep_closed_connections_until_completions_drain() {
    let channel = crate::grpc::zerocopy::ZeroCopyChannelMetrics::default();
    let metrics = ZeroCopyMetrics::default();
    metrics.0.closed.store(true, Ordering::Release);
    metrics.0.pending_sends.store(1, Ordering::Release);
    metrics.0.pending_bytes.store(1024, Ordering::Relaxed);
    let weak = Arc::downgrade(&metrics.0);
    channel.register(metrics.clone());
    assert_eq!(channel.snapshot().pending_sends, 1);
    metrics.0.pending_bytes.store(0, Ordering::Relaxed);
    metrics.0.copied.store(1024, Ordering::Relaxed);
    metrics.0.pending_sends.store(0, Ordering::Release);
    drop(metrics);
    assert_eq!(channel.snapshot().completed_with_copy_flag, 1024);
    assert!(weak.upgrade().is_none());
}

struct Owner {
    bytes: Vec<u8>,
    dropped: Arc<AtomicUsize>,
}
impl AsRef<[u8]> for Owner {
    fn as_ref(&self) -> &[u8] {
        &self.bytes
    }
}
impl Drop for Owner {
    fn drop(&mut self) {
        self.dropped.fetch_add(1, Ordering::Relaxed);
    }
}

fn shared() -> Arc<Shared> {
    Arc::new(Shared {
        state: Mutex::new(State {
            pending: VecDeque::new(),
            bytes: 0,
            next_id: 0,
            closed: false,
            failed: false,
            capacity_waker: None,
        }),
        activity: Condvar::new(),
        metrics: ZeroCopyMetrics::default(),
        adaptive_fallback: true,
    })
}

#[test]
fn worker_setup_failure_can_fall_back_before_any_submission() {
    let shared = shared();
    shared.lock().failed = true;
    shared.metrics.0.errors.store(1, Ordering::Relaxed);
    shared.metrics.0.enabled.store(true, Ordering::Relaxed);
    shared.fallback_before_use();
    assert!(!shared.lock().failed);
    let stats = shared.metrics.snapshot();
    assert!(!stats.enabled);
    assert_eq!(stats.completion_errors, 0);
    assert_eq!(stats.fallbacks, 1);
}

#[test]
fn completion_ranges_release_only_matching_owners_across_wrap_and_reordering() {
    let shared = shared();
    let dropped = Arc::new(AtomicUsize::new(0));
    for id in [u32::MAX - 1, u32::MAX, 0, 1, 2] {
        let data = Bytes::from_owner(Owner {
            bytes: vec![7; 16],
            dropped: Arc::clone(&dropped),
        });
        shared.lock().pending.push_back(Pending {
            id,
            head: Bytes::new(),
            data,
            sent: 16,
        });
    }
    shared.lock().bytes = 80;
    shared.metrics.0.enabled.store(true, Ordering::Relaxed);
    shared.completed(1, 1, false);
    assert_eq!(dropped.load(Ordering::Relaxed), 1);
    shared.completed(u32::MAX, 0, true);
    assert_eq!(dropped.load(Ordering::Relaxed), 3);
    let stats = shared.metrics.snapshot();
    assert_eq!(stats.pending_sends, 2);
    assert_eq!(stats.pending_bytes, 32);
    assert_eq!(stats.completed_with_copy_flag, 32);
    assert_eq!(stats.completed_without_copy_flag, 16);
    assert!(!stats.enabled);
    // Duplicate completions must not release other owners or double count bytes.
    shared.completed(u32::MAX, 0, false);
    assert_eq!(dropped.load(Ordering::Relaxed), 3);
    shared.completed(2, 2, false);
    shared.completed(u32::MAX - 1, u32::MAX - 1, false);
    assert_eq!(dropped.load(Ordering::Relaxed), 5);
    assert_eq!(shared.metrics.snapshot().pending_sends, 0);
}

#[test]
fn partial_send_retains_only_accepted_ranges_without_copying() {
    let head = Bytes::from(vec![1; 9]);
    let data = Bytes::from(vec![2; 4096]);
    for n in [1, 9, 10, 4096 + 9] {
        let mut pending = Pending {
            id: 0,
            head: head.clone(),
            data: data.clone(),
            sent: 0,
        };
        pending.truncate(n);
        assert_eq!(pending.head.len() + pending.data.len(), n);
        assert_eq!(pending.head.as_ptr(), head.as_ptr());
        if n > 9 {
            assert_eq!(pending.data.as_ptr(), data.as_ptr());
        }
    }
}

#[test]
fn closed_connection_still_retains_pending_owners_until_completion() {
    let shared = shared();
    let dropped = Arc::new(AtomicUsize::new(0));
    shared.lock().pending.push_back(Pending {
        id: 42,
        head: Bytes::new(),
        sent: 8,
        data: Bytes::from_owner(Owner {
            bytes: vec![1; 8],
            dropped: Arc::clone(&dropped),
        }),
    });
    shared.lock().bytes = 8;
    shared.lock().closed = true;
    assert_eq!(dropped.load(Ordering::Relaxed), 0);
    shared.completed(42, 42, false);
    assert_eq!(dropped.load(Ordering::Relaxed), 1);
}

fn notification(ipv6: bool) -> ([usize; 16], usize) {
    let mut control = [0usize; 16];
    // SAFETY: aligned storage has room for the header and sock_extended_err.
    // These Linux structures have no uninitialized padding in this construction.
    let len = unsafe {
        let header = control.as_mut_ptr().cast::<libc::cmsghdr>();
        let len = libc::CMSG_LEN(size_of::<libc::sock_extended_err>() as u32) as usize;
        header.write(libc::cmsghdr {
            cmsg_len: len,
            cmsg_level: if ipv6 { libc::SOL_IPV6 } else { libc::SOL_IP },
            cmsg_type: if ipv6 { libc::IPV6_RECVERR } else { libc::IP_RECVERR },
        });
        libc::CMSG_DATA(header).cast::<libc::sock_extended_err>().write(libc::sock_extended_err {
            ee_errno: 0,
            ee_origin: SO_EE_ORIGIN_ZEROCOPY,
            ee_type: 0,
            ee_code: SO_EE_CODE_ZEROCOPY_COPIED,
            ee_pad: 0,
            ee_info: 3,
            ee_data: 3,
        });
        len
    };
    (control, len)
}

#[test]
fn parses_ipv4_ipv6_notifications_and_rejects_malformed_control_data() {
    for ipv6 in [false, true] {
        let shared = shared();
        shared.lock().pending.push_back(Pending {
            id: 3,
            head: Bytes::new(),
            data: Bytes::from_static(b"hello"),
            sent: 5,
        });
        shared.lock().bytes = 5;
        let (mut control, len) = notification(ipv6);
        assert!(parse_control(&control, len, libc::MSG_CTRUNC, &shared).is_err());
        assert!(parse_control(&control, size_of_val(&control) + 1, 0, &shared).is_err());
        assert_eq!(shared.lock().pending.len(), 1);
        parse_control(&control, len, 0, &shared).unwrap();
        assert_eq!(shared.metrics.snapshot().completed_with_copy_flag, 5);
        // cmsg_len is the leading size_t in the Linux cmsghdr ABI.
        control[0] = usize::MAX;
        assert!(parse_control(&control, len, 0, &shared).is_err());
        control[0] = 1;
        assert!(parse_control(&control, len, 0, &shared).is_err());
        control[0] = size_of::<libc::cmsghdr>();
        assert!(parse_control(&control, len, 0, &shared).is_err());
    }
}

// Intentionally leaks a fake outstanding owner, like the production fail-closed
// path. Excluded from Miri's leak checker; no actual kernel references are used.
#[test]
#[cfg(not(miri))]
fn failed_worker_quarantines_unreleased_memory() {
    let shared = shared();
    let dropped = Arc::new(AtomicUsize::new(0));
    shared.lock().pending.push_back(Pending {
        id: 0,
        head: Bytes::new(),
        sent: 8,
        data: Bytes::from_owner(Owner {
            bytes: vec![1; 8],
            dropped: Arc::clone(&dropped),
        }),
    });
    shared.lock().bytes = 8;
    drop(CompletionGuard {
        fd: None,
        shared: Some(Arc::clone(&shared)),
    });
    assert_eq!(dropped.load(Ordering::Relaxed), 0);
    assert_eq!(shared.metrics.snapshot().completion_errors, 1);
    assert!(shared.lock().failed);
}
