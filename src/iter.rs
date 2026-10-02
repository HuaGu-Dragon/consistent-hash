use std::hash::{BuildHasher, Hash};

use crate::{ConsistentHash, snapshot::NodeRef};

pub struct Iter<'a, K, N, S, const R: usize> {
    pub(crate) inner: &'a ConsistentHash<K, N, S, R>,
    pub(crate) hash: u64,
}

impl<'a, K, N, S, const R: usize> Iterator for Iter<'a, K, N, S, R>
where
    N: Clone,
    K: Hash,
    S: BuildHasher,
{
    type Item = NodeRef<N>;

    fn next(&mut self) -> Option<Self::Item> {
        let (hash, node) = self.inner.search_nearest(self.hash)?;
        if hash < self.hash {
            return None;
        }
        self.hash = hash + 1;

        Some(node)
    }
}
