use std::{ops::Deref, ptr::NonNull, sync::Arc};

use crate::inner::{Inner, Token};

#[derive(Debug, Clone)]
pub struct NodeRef<N> {
    _snapshot: Arc<Inner<N>>,
    node: NonNull<N>,
}

unsafe impl<N: Send + Sync> Send for NodeRef<N> {}
unsafe impl<N: Sync> Sync for NodeRef<N> {}

impl<N> Deref for NodeRef<N> {
    type Target = N;

    fn deref(&self) -> &Self::Target {
        // SAFETY: The node is guaranteed to be valid as long as the snapshot is alive.
        unsafe { self.node.as_ref() }
    }
}

impl<N> NodeRef<N> {
    pub(crate) fn new(snapshot: Arc<Inner<N>>, token: Token) -> Self {
        let node = unsafe { NonNull::new_unchecked(&raw const snapshot[token] as *mut _) };

        Self {
            _snapshot: snapshot,
            node,
        }
    }
}
