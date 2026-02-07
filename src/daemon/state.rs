use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use tokio::sync::Semaphore;
use tokio_util::sync::CancellationToken;

use crate::engine::CompletionEngine;
use crate::storage::{StorageEvent, StorageEventSender};

/// Maximum number of concurrent connections the daemon will accept.
pub const MAX_CONCURRENT_CONNECTIONS: usize = 100;

/// Shared state for the daemon, passed to every connection handler.
#[derive(Clone)]
pub struct DaemonState {
    /// The completion backend.
    pub engine: Arc<dyn CompletionEngine>,

    /// Limits concurrent connections.
    pub semaphore: Arc<Semaphore>,

    /// Signals shutdown across all tasks.
    pub cancel: CancellationToken,

    /// Total requests processed since startup.
    pub total_requests: Arc<AtomicU64>,

    /// Currently active connection count.
    pub active_connections: Arc<AtomicU64>,

    /// Optional storage event sender for persistence.
    pub storage: Option<StorageEventSender>,

    /// Session ID for correlating events across tables.
    pub session_id: String,
}

impl DaemonState {
    /// Create a new `DaemonState` with the given engine.
    pub fn new(engine: Arc<dyn CompletionEngine>) -> Self {
        Self {
            engine,
            semaphore: Arc::new(Semaphore::new(MAX_CONCURRENT_CONNECTIONS)),
            cancel: CancellationToken::new(),
            total_requests: Arc::new(AtomicU64::new(0)),
            active_connections: Arc::new(AtomicU64::new(0)),
            storage: None,
            session_id: String::new(),
        }
    }

    /// Set the storage event sender.
    #[must_use]
    pub fn with_storage(mut self, sender: StorageEventSender) -> Self {
        self.storage = Some(sender);
        self
    }

    /// Set the session ID.
    #[must_use]
    pub fn with_session_id(mut self, session_id: String) -> Self {
        self.session_id = session_id;
        self
    }

    /// Emit a storage event (non-blocking, best-effort).
    ///
    /// Uses `try_send()` to avoid awaiting on the channel. If the channel is
    /// full or absent, the event is dropped with a warning — storage events are
    /// observability data, not business-critical.
    pub fn emit_storage_event(&self, event: StorageEvent) {
        if let Some(ref sender) = self.storage
            && let Err(e) = sender.try_send(event)
        {
            tracing::warn!("storage event dropped: {e}");
        }
    }

    /// Increment active connections, returning a guard that decrements on drop.
    #[must_use = "dropping the guard immediately decrements active_connections"]
    pub fn connection_guard(&self) -> ConnectionGuard {
        self.active_connections.fetch_add(1, Ordering::Relaxed);
        ConnectionGuard {
            counter: Arc::clone(&self.active_connections),
        }
    }

    /// Increment total requests counter.
    pub fn record_request(&self) {
        self.total_requests.fetch_add(1, Ordering::Relaxed);
    }
}

/// RAII guard that decrements `active_connections` when dropped.
pub struct ConnectionGuard {
    counter: Arc<AtomicU64>,
}

impl Drop for ConnectionGuard {
    fn drop(&mut self) {
        self.counter.fetch_sub(1, Ordering::Relaxed);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::StubEngine;

    fn make_state() -> DaemonState {
        DaemonState::new(Arc::new(StubEngine))
    }

    #[test]
    fn connection_guard_increments_and_decrements() {
        let state = make_state();
        assert_eq!(state.active_connections.load(Ordering::Relaxed), 0);

        let guard = state.connection_guard();
        assert_eq!(state.active_connections.load(Ordering::Relaxed), 1);

        drop(guard);
        assert_eq!(state.active_connections.load(Ordering::Relaxed), 0);
    }

    #[test]
    fn multiple_guards() {
        let state = make_state();
        let g1 = state.connection_guard();
        let g2 = state.connection_guard();
        assert_eq!(state.active_connections.load(Ordering::Relaxed), 2);

        drop(g1);
        assert_eq!(state.active_connections.load(Ordering::Relaxed), 1);

        drop(g2);
        assert_eq!(state.active_connections.load(Ordering::Relaxed), 0);
    }

    #[test]
    fn record_request_increments() {
        let state = make_state();
        state.record_request();
        state.record_request();
        assert_eq!(state.total_requests.load(Ordering::Relaxed), 2);
    }

    #[test]
    fn semaphore_has_correct_permits() {
        let state = make_state();
        assert_eq!(
            state.semaphore.available_permits(),
            MAX_CONCURRENT_CONNECTIONS
        );
    }
}
