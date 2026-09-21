use std::{ops::Deref, sync::Arc};

use crate::inner::Inner;

#[derive(Clone)]
pub struct NodeRef<N> {
    pub(crate) _snapshot: Arc<Inner<N>>,
    pub(crate) node: *const N,
}

unsafe impl<N: Send + Sync> Send for NodeRef<N> {}
unsafe impl<N: Sync> Sync for NodeRef<N> {}

impl<N> Deref for NodeRef<N> {
    type Target = N;

    fn deref(&self) -> &Self::Target {
        unsafe { &*self.node }
    }
}
