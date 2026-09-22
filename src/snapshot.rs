use std::{ops::Deref, sync::Arc};

use crate::inner::Inner;

#[derive(Clone)]
pub struct NodeRef<N> {
    _snapshot: Arc<Inner<N>>,
    node: *const N,
}

unsafe impl<N: Send + Sync> Send for NodeRef<N> {}
unsafe impl<N: Sync> Sync for NodeRef<N> {}

impl<N> Deref for NodeRef<N> {
    type Target = N;

    fn deref(&self) -> &Self::Target {
        unsafe { &*self.node }
    }
}

impl<N> NodeRef<N> {
    pub(crate) fn new(snapshot: Arc<Inner<N>>, token: usize) -> Self {
        let node = &raw const snapshot.nodes[token];

        Self {
            _snapshot: snapshot,
            node,
        }
    }
}
