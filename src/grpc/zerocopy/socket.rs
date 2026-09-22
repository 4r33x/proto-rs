use std::collections::VecDeque;
use std::io;
use std::os::fd::AsFd;
use std::os::fd::AsRawFd;
use std::os::fd::OwnedFd;
use std::os::fd::RawFd;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::Condvar;
use std::sync::Mutex;
use std::sync::MutexGuard;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::AtomicU64;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;
use std::task::Context;
use std::task::Poll;
use std::task::Waker;
use std::task::ready;

use bytes::Bytes;
use hyper_zerocopy::rt::Read;
use hyper_zerocopy::rt::ReadBufCursor;
use hyper_zerocopy::rt::Write;
use tokio::io::AsyncRead;
use tokio::io::AsyncWrite;
use tokio::io::Interest;
use tokio::net::TcpStream;
use tokio::sync::Notify;

// Linux UAPI linux/errqueue.h; these constants are not exposed by libc 0.2.189.
const SO_EE_ORIGIN_ZEROCOPY: u8 = 5;
const SO_EE_CODE_ZEROCOPY_COPIED: u8 = 1;

/// Controls speculative Linux zero-copy sends, not gRPC message limits.
#[derive(Clone, Copy, Debug)]
pub struct ZeroCopyConfig {
    /// Smaller writes use ordinary copying sends. Default: 16 KiB.
    pub min_send_bytes: usize,
    /// Maximum outstanding successful MSG_ZEROCOPY calls. Default: 256.
    pub max_in_flight_sends: usize,
    /// Maximum outstanding submitted byte ranges. Default: 8 MiB.
    /// A Bytes slice can retain a larger backing allocation; this is not an RSS limit.
    pub max_in_flight_bytes: usize,
    /// Permit ordinary writes if SO_ZEROCOPY or a zero-copy send is unavailable.
    pub allow_fallback: bool,
    /// Stop requesting zero-copy when the kernel reports a copied completion.
    /// Disable only to exercise the completion path on loopback in tests/benchmarks.
    pub adaptive_fallback: bool,
}

impl Default for ZeroCopyConfig {
    fn default() -> Self {
        Self {
            min_send_bytes: 16 * 1024,
            max_in_flight_sends: 256,
            max_in_flight_bytes: 8 * 1024 * 1024,
            allow_fallback: true,
            adaptive_fallback: true,
        }
    }
}

impl ZeroCopyConfig {
    pub(super) fn validate(self) -> io::Result<()> {
        if self.max_in_flight_sends == 0
            || self.max_in_flight_sends > 65536
            || self.min_send_bytes == 0
            || self.max_in_flight_bytes < self.min_send_bytes
        {
            return Err(io::Error::new(io::ErrorKind::InvalidInput, "invalid zero-copy queue limits"));
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct ZeroCopySnapshot {
    pub enabled: bool,
    pub send_calls: u64,
    pub submitted_bytes: u64,
    /// Completed ranges without SO_EE_CODE_ZEROCOPY_COPIED, not a NIC DMA counter.
    pub completed_without_copy_flag: u64,
    /// Conservatively includes entire ranges carrying the copied/mixed flag.
    pub completed_with_copy_flag: u64,
    pub ordinary_send_bytes: u64,
    pub fallbacks: u64,
    pub completion_errors: u64,
    pub pending_sends: usize,
    pub pending_bytes: usize,
}

#[derive(Default)]
struct Counters {
    closed: AtomicBool,
    enabled: AtomicBool,
    send_calls: AtomicU64,
    submitted: AtomicU64,
    completed: AtomicU64,
    copied: AtomicU64,
    ordinary: AtomicU64,
    fallbacks: AtomicU64,
    errors: AtomicU64,
    pending_sends: AtomicUsize,
    pending_bytes: AtomicUsize,
    idle: Notify,
    fallback_warning: std::sync::OnceLock<Arc<std::sync::OnceLock<&'static str>>>,
}

/// Per-connection counters; snapshots are observational, not atomic transactions.
#[derive(Clone, Default)]
pub struct ZeroCopyMetrics(Arc<Counters>);

impl ZeroCopyMetrics {
    pub(super) fn is_closed(&self) -> bool {
        self.0.closed.load(Ordering::Acquire)
    }
    pub(crate) fn set_fallback_warning(&self, warning: Arc<std::sync::OnceLock<&'static str>>) {
        let _ = self.0.fallback_warning.set(warning);
    }

    fn warn_fallback(&self) {
        if let Some(warning) = self.0.fallback_warning.get() {
            crate::grpc::channel::warn_once(warning, "kernel selected ordinary copying sends");
        }
    }
    pub fn snapshot(&self) -> ZeroCopySnapshot {
        let c = &self.0;
        ZeroCopySnapshot {
            enabled: c.enabled.load(Ordering::Relaxed),
            send_calls: c.send_calls.load(Ordering::Relaxed),
            submitted_bytes: c.submitted.load(Ordering::Relaxed),
            completed_without_copy_flag: c.completed.load(Ordering::Relaxed),
            completed_with_copy_flag: c.copied.load(Ordering::Relaxed),
            ordinary_send_bytes: c.ordinary.load(Ordering::Relaxed),
            fallbacks: c.fallbacks.load(Ordering::Relaxed),
            completion_errors: c.errors.load(Ordering::Relaxed),
            pending_sends: c.pending_sends.load(Ordering::Acquire),
            pending_bytes: c.pending_bytes.load(Ordering::Relaxed),
        }
    }

    /// Wait for outstanding kernel buffer references to be released. This does
    /// not mean the peer application received the data. Stop producers first;
    /// wrap this in a timeout if the peer/network can stall.
    pub async fn wait_for_idle(&self) -> io::Result<()> {
        loop {
            let notified = self.0.idle.notified();
            tokio::pin!(notified);
            notified.as_mut().enable();
            if self.0.errors.load(Ordering::Acquire) != 0 {
                return Err(io::Error::other(
                    "zero-copy completion reader failed; outstanding buffers quarantined",
                ));
            }
            if self.0.pending_sends.load(Ordering::Acquire) == 0 {
                return Ok(());
            }
            notified.await;
        }
    }
}

struct Pending {
    id: u32,
    head: Bytes,
    data: Bytes,
    sent: usize,
}

impl Pending {
    fn truncate(&mut self, sent: usize) {
        let header = sent.min(self.head.len());
        self.head.truncate(header);
        if sent == header {
            self.data = Bytes::new();
        } else {
            self.data.truncate(sent - header);
        }
        self.sent = sent;
    }
}

struct State {
    pending: VecDeque<Pending>,
    bytes: usize,
    next_id: u32,
    closed: bool,
    failed: bool,
    capacity_waker: Option<Waker>,
}

struct Shared {
    state: Mutex<State>,
    activity: Condvar,
    metrics: ZeroCopyMetrics,
    adaptive_fallback: bool,
}

impl Shared {
    // Constructor-only recovery: no I/O has been exposed and no submission can
    // exist. A failed thread spawn drops its CompletionGuard, which deliberately
    // marks failure; here ordinary writes can safely replace a missing worker.
    fn fallback_before_use(&self) {
        let mut state = self.lock();
        assert!(state.pending.is_empty() && state.next_id == 0);
        state.failed = false;
        self.metrics.0.errors.store(0, Ordering::Relaxed);
        self.metrics.0.enabled.store(false, Ordering::Relaxed);
        self.metrics.0.fallbacks.fetch_add(1, Ordering::Relaxed);
    }

    fn lock(&self) -> MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    fn completed(&self, first: u32, last: u32, copied: bool) {
        let mut state = self.lock();
        let mut released = 0;
        // Inclusive modular interval; handles wraparound and out-of-order ranges.
        state.pending.retain(|entry| {
            if entry.id.wrapping_sub(first) <= last.wrapping_sub(first) {
                released += entry.sent;
                false
            } else {
                true
            }
        });
        state.bytes -= released;
        let metrics = &self.metrics.0;
        let counter = if copied { &metrics.copied } else { &metrics.completed };
        counter.fetch_add(released as u64, Ordering::Relaxed);
        let fell_back = copied && self.adaptive_fallback && metrics.enabled.swap(false, Ordering::Relaxed);
        if fell_back {
            metrics.fallbacks.fetch_add(1, Ordering::Relaxed);
        }
        metrics.pending_bytes.store(state.bytes, Ordering::Relaxed);
        metrics.pending_sends.store(state.pending.len(), Ordering::Release);
        let waker = state.capacity_waker.take();
        drop(state);
        metrics.idle.notify_waiters();
        if fell_back {
            self.metrics.warn_fallback();
        }
        if let Some(waker) = waker {
            waker.wake();
        }
    }
}

/// Plain TCP HTTP/2 I/O with owned-buffer MSG_ZEROCOPY writes.
///
/// A dedicated completion thread owns a duplicate descriptor and retained Bytes.
/// It survives connection cancellation and Tokio runtime shutdown. No borrowed
/// AsyncWrite input is ever submitted using MSG_ZEROCOPY.
pub struct ZeroCopyIo {
    socket: TcpStream,
    shared: Arc<Shared>,
    config: ZeroCopyConfig,
}

impl ZeroCopyIo {
    /// Open a fresh plain TCP connection. No TLS negotiation is performed.
    pub async fn connect(address: impl tokio::net::ToSocketAddrs, config: ZeroCopyConfig) -> io::Result<Self> {
        Self::new(TcpStream::connect(address).await?, config)
    }

    /// Accept a fresh plain TCP connection without exposing its descriptor.
    pub async fn accept(listener: &tokio::net::TcpListener, config: ZeroCopyConfig) -> io::Result<(Self, std::net::SocketAddr)> {
        let (socket, address) = listener.accept().await?;
        Ok((Self::new(socket, config)?, address))
    }

    // Only fresh sockets may enter here: the kernel's per-socket zero-copy
    // cookie must start at zero and this transport exclusively consumes its
    // error queue. Do not expose an unchecked safe from-stream constructor.
    fn new(socket: TcpStream, config: ZeroCopyConfig) -> io::Result<Self> {
        socket.set_nodelay(true)?;
        Self::from_connector(socket, config, false)
    }

    // Fresh sockets only, created by the configured Tonic TCP connector. Keep
    // its socket options (including an explicit TCP_NODELAY=false) unchanged.
    pub(super) fn from_connector(socket: TcpStream, config: ZeroCopyConfig, ordinary: bool) -> io::Result<Self> {
        config.validate()?;
        let enabled = match if ordinary {
            Ok(false)
        } else {
            enable_zerocopy(socket.as_raw_fd()).map(|()| true)
        } {
            Ok(enabled) => enabled,
            Err(_) if config.allow_fallback => false,
            Err(error) => return Err(error),
        };
        let shared = Arc::new(Shared {
            state: Mutex::new(State {
                pending: VecDeque::with_capacity(config.max_in_flight_sends),
                bytes: 0,
                next_id: 0,
                closed: false,
                failed: false,
                capacity_waker: None,
            }),
            activity: Condvar::new(),
            metrics: ZeroCopyMetrics::default(),
            adaptive_fallback: config.adaptive_fallback && config.allow_fallback,
        });
        shared.metrics.0.enabled.store(enabled, Ordering::Relaxed);
        if enabled {
            let setup = socket.as_fd().try_clone_to_owned().and_then(|fd| {
                let guard = CompletionGuard {
                    fd: Some(fd),
                    shared: Some(Arc::clone(&shared)),
                };
                std::thread::Builder::new().name("proto-rs-zc".into()).spawn(move || completion_loop(guard)).map(|_| ())
            });
            if let Err(error) = setup {
                if !config.allow_fallback {
                    return Err(error);
                }
                shared.fallback_before_use();
            }
        } else {
            shared.metrics.0.fallbacks.fetch_add(1, Ordering::Relaxed);
        }
        Ok(Self { socket, shared, config })
    }

    pub fn metrics(&self) -> ZeroCopyMetrics {
        self.shared.metrics.clone()
    }

    pub(super) const fn config(&self) -> ZeroCopyConfig {
        self.config
    }

    fn poll_owned(&mut self, cx: &mut Context<'_>, head: &Bytes, data: &Bytes) -> Poll<io::Result<usize>> {
        if head.is_empty() && data.is_empty() {
            return Poll::Ready(Ok(0));
        }
        loop {
            ready!(self.socket.poll_write_ready(cx))?;
            let mut state = self.shared.lock();
            if state.failed {
                return Poll::Ready(Err(io::Error::other("zero-copy completion reader failed")));
            }
            let metrics = &self.shared.metrics.0;
            let total = head.len().saturating_add(data.len());
            if !metrics.enabled.load(Ordering::Relaxed) || total < self.config.min_send_bytes {
                drop(state);
                let result = self.socket.try_io(Interest::WRITABLE, || send(self.socket.as_raw_fd(), head, data, false));
                match result {
                    Ok(n) => {
                        metrics.ordinary.fetch_add(n as u64, Ordering::Relaxed);
                        return Poll::Ready(Ok(n));
                    }
                    Err(error) if error.kind() == io::ErrorKind::WouldBlock || error.kind() == io::ErrorKind::Interrupted => continue,
                    Err(error) => return Poll::Ready(Err(error)),
                }
            }
            let available = self.config.max_in_flight_bytes - state.bytes;
            if state.pending.len() == self.config.max_in_flight_sends
                || available < self.config.min_send_bytes
                || state.pending.front().is_some_and(|entry| state.next_id.wrapping_sub(entry.id) >= (1 << 31))
            {
                if !state.capacity_waker.as_ref().is_some_and(|w| w.will_wake(cx.waker())) {
                    state.capacity_waker = Some(cx.waker().clone());
                }
                return Poll::Pending;
            }
            // Clone/slice BEFORE the syscall, so allocation failure cannot leave
            // a successful kernel submission without an owner. Queue capacity is
            // preallocated, and its lock spans syscall + cookie registration.
            let limit = available.min(total);
            let header = head.len().min(limit);
            let mut pending = Pending {
                id: state.next_id,
                head: head.slice(..header),
                data: if limit > header {
                    data.slice(..limit - header)
                } else {
                    Bytes::new()
                },
                sent: 0,
            };
            let result = self.socket.try_io(Interest::WRITABLE, || {
                send(self.socket.as_raw_fd(), &pending.head, &pending.data, true)
            });
            match result {
                Ok(0) => return Poll::Ready(Err(io::ErrorKind::WriteZero.into())),
                Ok(n) => {
                    pending.truncate(n);
                    state.pending.push_back(pending);
                    state.next_id = state.next_id.wrapping_add(1);
                    state.bytes += n;
                    metrics.send_calls.fetch_add(1, Ordering::Relaxed);
                    metrics.submitted.fetch_add(n as u64, Ordering::Relaxed);
                    metrics.pending_bytes.store(state.bytes, Ordering::Relaxed);
                    metrics.pending_sends.store(state.pending.len(), Ordering::Release);
                    self.shared.activity.notify_one();
                    return Poll::Ready(Ok(n));
                }
                Err(error) if error.kind() == io::ErrorKind::WouldBlock || error.kind() == io::ErrorKind::Interrupted => {}
                Err(error)
                    if self.config.allow_fallback
                        && matches!(
                            error.raw_os_error(),
                            Some(libc::ENOBUFS | libc::EOPNOTSUPP | libc::EINVAL | libc::ENOSYS)
                        ) =>
                {
                    metrics.enabled.store(false, Ordering::Relaxed);
                    metrics.fallbacks.fetch_add(1, Ordering::Relaxed);
                    drop(state);
                    self.shared.metrics.warn_fallback();
                }
                Err(error) => return Poll::Ready(Err(error)),
            }
        }
    }
}

impl Drop for ZeroCopyIo {
    fn drop(&mut self) {
        // Stop new traffic; the completion thread still owns an open duplicate
        // and all submitted memory until the kernel releases its references.
        unsafe {
            libc::shutdown(self.socket.as_raw_fd(), libc::SHUT_RDWR);
        }
        self.shared.lock().closed = true;
        self.shared.metrics.0.closed.store(true, Ordering::Release);
        self.shared.activity.notify_one();
    }
}

impl Read for ZeroCopyIo {
    fn poll_read(mut self: Pin<&mut Self>, cx: &mut Context<'_>, mut buf: ReadBufCursor<'_>) -> Poll<io::Result<()>> {
        // SAFETY: ReadBuf treats the unfilled memory as uninitialized. Only its
        // initialized/filled prefix is committed back to Hyper after success.
        let n = unsafe {
            let mut read = tokio::io::ReadBuf::uninit(buf.as_mut());
            ready!(Pin::new(&mut self.socket).poll_read(cx, &mut read))?;
            read.filled().len()
        };
        unsafe {
            buf.advance(n);
        }
        Poll::Ready(Ok(()))
    }
}

impl Write for ZeroCopyIo {
    fn is_write_owned(&self) -> bool {
        true
    }
    fn poll_write_owned(mut self: Pin<&mut Self>, cx: &mut Context<'_>, head: &Bytes, data: &Bytes) -> Poll<io::Result<usize>> {
        self.poll_owned(cx, head, data)
    }
    fn poll_write(mut self: Pin<&mut Self>, cx: &mut Context<'_>, bytes: &[u8]) -> Poll<io::Result<usize>> {
        let n = ready!(Pin::new(&mut self.socket).poll_write(cx, bytes))?;
        self.shared.metrics.0.ordinary.fetch_add(n as u64, Ordering::Relaxed);
        Poll::Ready(Ok(n))
    }
    fn is_write_vectored(&self) -> bool {
        true
    }
    fn poll_write_vectored(mut self: Pin<&mut Self>, cx: &mut Context<'_>, bufs: &[io::IoSlice<'_>]) -> Poll<io::Result<usize>> {
        let n = ready!(Pin::new(&mut self.socket).poll_write_vectored(cx, bufs))?;
        self.shared.metrics.0.ordinary.fetch_add(n as u64, Ordering::Relaxed);
        Poll::Ready(Ok(n))
    }
    // Flush means accepted by TCP, not wait-for-ACK. Waiting for zero-copy
    // completions after every HTTP/2 frame would serialize the network pipeline.
    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.socket).poll_flush(cx)
    }
    fn poll_shutdown(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.socket).poll_shutdown(cx)
    }
}

fn enable_zerocopy(fd: RawFd) -> io::Result<()> {
    let enabled: libc::c_int = 1;
    // SAFETY: fd is live; enabled is correctly sized/aligned for this option.
    let result = unsafe {
        libc::setsockopt(
            fd,
            libc::SOL_SOCKET,
            libc::SO_ZEROCOPY,
            std::ptr::from_ref(&enabled).cast(),
            size_of_val(&enabled) as libc::socklen_t,
        )
    };
    if result == 0 { Ok(()) } else { Err(io::Error::last_os_error()) }
}

fn send(fd: RawFd, head: &[u8], data: &[u8], zerocopy: bool) -> io::Result<usize> {
    let mut iov = [
        libc::iovec {
            iov_base: head.as_ptr().cast_mut().cast(),
            iov_len: head.len(),
        },
        libc::iovec {
            iov_base: data.as_ptr().cast_mut().cast(),
            iov_len: data.len(),
        },
    ];
    // SAFETY: zero is valid for all msghdr fields. sendmsg reads the two valid
    // slices synchronously; the owned path separately retains their allocations
    // until the kernel's zero-copy completion notification.
    let mut msg: libc::msghdr = unsafe { std::mem::zeroed() };
    msg.msg_iov = iov.as_mut_ptr();
    msg.msg_iovlen = iov.len();
    let flags = libc::MSG_DONTWAIT | libc::MSG_NOSIGNAL | if zerocopy { libc::MSG_ZEROCOPY } else { 0 };
    let n = unsafe { libc::sendmsg(fd, &raw const msg, flags) };
    if n < 0 { Err(io::Error::last_os_error()) } else { Ok(n as usize) }
}

/// RAII fail-closed ownership. If the worker fails or unwinds with unresolved
/// kernel references, quarantine (leak) its descriptor and owners instead of
/// permitting reuse. This exceptional path is observable through metrics.
struct CompletionGuard {
    fd: Option<OwnedFd>,
    shared: Option<Arc<Shared>>,
}

impl Drop for CompletionGuard {
    fn drop(&mut self) {
        let shared = self.shared.as_ref().unwrap();
        let mut state = shared.lock();
        let failed = !state.closed || !state.pending.is_empty();
        state.failed = failed;
        let quarantine = !state.pending.is_empty();
        let waker = state.capacity_waker.take();
        if failed {
            shared.metrics.0.errors.fetch_add(1, Ordering::Release);
            shared.metrics.0.enabled.store(false, Ordering::Relaxed);
        }
        drop(state);
        shared.metrics.0.idle.notify_waiters();
        if quarantine {
            std::mem::forget(self.fd.take());
            std::mem::forget(self.shared.take());
        }
        if let Some(waker) = waker {
            waker.wake();
        }
    }
}

#[allow(clippy::needless_pass_by_value)] // owns the RAII quarantine guard, including on unwind
fn completion_loop(guard: CompletionGuard) {
    let shared = guard.shared.as_ref().unwrap();
    let fd = guard.fd.as_ref().unwrap().as_raw_fd();
    let mut woke_without_progress = false;
    loop {
        let mut state = shared.lock();
        while state.pending.is_empty() && !state.closed {
            state = shared.activity.wait(state).unwrap_or_else(std::sync::PoisonError::into_inner);
        }
        if state.pending.is_empty() {
            return;
        }
        drop(state);
        match drain_completions(fd, shared) {
            Ok(true) => {
                woke_without_progress = false;
                continue;
            }
            Ok(false) => {
                // A sticky socket error/HUP can make poll immediately ready
                // without an error-queue entry. Avoid spinning on teardown.
                if woke_without_progress {
                    std::thread::sleep(std::time::Duration::from_millis(10));
                }
            }
            Err(_) => return, // CompletionGuard retains unresolved owners.
        }
        let mut pollfd = libc::pollfd { fd, events: 0, revents: 0 };
        // SAFETY: one valid pollfd. POLLERR is reported even with events=0.
        let result = unsafe { libc::poll(&raw mut pollfd, 1, -1) };
        if result < 0 && io::Error::last_os_error().kind() != io::ErrorKind::Interrupted {
            return;
        }
        if pollfd.revents & libc::POLLNVAL != 0 {
            return;
        }
        // Never interpret HUP as permission to release submitted memory.
        woke_without_progress = result > 0;
    }
}

fn drain_completions(fd: RawFd, shared: &Shared) -> io::Result<bool> {
    let mut read_any = false;
    loop {
        // cmsghdr alignment is no greater than usize alignment on Linux ABIs.
        let mut control = [0usize; 128];
        let mut msg: libc::msghdr = unsafe { std::mem::zeroed() };
        msg.msg_control = control.as_mut_ptr().cast();
        msg.msg_controllen = size_of_val(&control);
        // SAFETY: the header points to writable aligned ancillary storage. No
        // payload buffer is needed for MSG_ERRQUEUE completion notifications.
        let n = unsafe { libc::recvmsg(fd, &raw mut msg, libc::MSG_ERRQUEUE | libc::MSG_DONTWAIT) };
        if n < 0 {
            let error = io::Error::last_os_error();
            if error.kind() == io::ErrorKind::WouldBlock {
                return Ok(read_any);
            }
            if error.kind() == io::ErrorKind::Interrupted {
                continue;
            }
            return Err(error);
        }
        read_any = true;
        parse_control(&control, msg.msg_controllen, msg.msg_flags, shared)?;
    }
}

fn parse_control(control: &[usize], len: usize, flags: libc::c_int, shared: &Shared) -> io::Result<()> {
    const {
        assert!(align_of::<libc::cmsghdr>() <= align_of::<usize>());
    }
    if flags & libc::MSG_CTRUNC != 0 || len > size_of_val(control) {
        return Err(io::Error::other("truncated zero-copy completion"));
    }
    let mut msg: libc::msghdr = unsafe { std::mem::zeroed() };
    msg.msg_control = control.as_ptr().cast_mut().cast();
    msg.msg_controllen = len;
    // SAFETY: CMSG helpers only read the aligned input; validate each header's
    // extent before reading its payload or using its length to find the next.
    unsafe {
        let mut cmsg = libc::CMSG_FIRSTHDR(&raw const msg);
        while !cmsg.is_null() {
            let header = &*cmsg;
            let offset = cmsg.cast::<u8>().offset_from(control.as_ptr().cast()) as usize;
            if header.cmsg_len < libc::CMSG_LEN(0) as usize || header.cmsg_len > len - offset {
                return Err(io::Error::other("invalid ancillary header length"));
            }
            if (header.cmsg_level == libc::SOL_IP && header.cmsg_type == libc::IP_RECVERR)
                || (header.cmsg_level == libc::SOL_IPV6 && header.cmsg_type == libc::IPV6_RECVERR)
            {
                if header.cmsg_len < libc::CMSG_LEN(size_of::<libc::sock_extended_err>() as u32) as usize {
                    return Err(io::Error::other("short zero-copy completion"));
                }
                let error = std::ptr::read_unaligned(libc::CMSG_DATA(cmsg).cast::<libc::sock_extended_err>());
                if error.ee_origin == SO_EE_ORIGIN_ZEROCOPY {
                    if error.ee_errno != 0
                        || error.ee_code > SO_EE_CODE_ZEROCOPY_COPIED
                        || error.ee_data.wrapping_sub(error.ee_info) >= (1 << 31)
                    {
                        return Err(io::Error::other("invalid zero-copy completion"));
                    }
                    shared.completed(error.ee_info, error.ee_data, error.ee_code == SO_EE_CODE_ZEROCOPY_COPIED);
                }
            }
            cmsg = libc::CMSG_NXTHDR(&raw const msg, cmsg);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests;
