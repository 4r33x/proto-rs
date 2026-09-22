use std::hash::Hash;
use std::pin::Pin;
use std::task::Context;
use std::task::Poll;

use tokio::sync::mpsc::Receiver;
use tokio_stream::Stream;
use tower::discover::Change as TowerChange;

use super::super::Connection;
use super::super::Endpoint;

/// A change in the service set.
#[derive(Debug, Clone)]
pub enum Change<K, V> {
    /// A new service identified by key `K` was identified.
    Insert(K, V),
    /// The service identified by key `K` disappeared.
    Remove(K),
}

pub(crate) struct DynamicServiceStream<K: Hash + Eq + Clone> {
    changes: Receiver<Change<K, Endpoint>>,
    factory: Option<Box<dyn FnMut(Endpoint) -> Connection + Send>>,
}

impl<K: Hash + Eq + Clone> DynamicServiceStream<K> {
    pub(crate) fn new(changes: Receiver<Change<K, Endpoint>>) -> Self {
        Self { changes, factory: None }
    }

    #[cfg(feature = "proto-rs-owned")]
    pub(crate) fn with_factory(
        changes: Receiver<Change<K, Endpoint>>,
        factory: impl FnMut(Endpoint) -> Connection + Send + 'static,
    ) -> Self {
        Self {
            changes,
            factory: Some(Box::new(factory)),
        }
    }
}

impl<K: Hash + Eq + Clone> Stream for DynamicServiceStream<K> {
    type Item = Result<TowerChange<K, Connection>, crate::BoxError>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        match Pin::new(&mut self.changes).poll_recv(cx) {
            Poll::Pending | Poll::Ready(None) => Poll::Pending,
            Poll::Ready(Some(change)) => match change {
                Change::Insert(k, endpoint) => {
                    let connection = match &mut self.factory {
                        Some(factory) => factory(endpoint),
                        None => Connection::lazy_endpoint(endpoint),
                    };
                    Poll::Ready(Some(Ok(TowerChange::Insert(k, connection))))
                }
                Change::Remove(k) => Poll::Ready(Some(Ok(TowerChange::Remove(k)))),
            },
        }
    }
}

impl<K: Hash + Eq + Clone> Unpin for DynamicServiceStream<K> {}
