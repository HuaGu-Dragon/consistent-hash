use std::{
    borrow::Borrow,
    hash::{BuildHasher, BuildHasherDefault, Hash, Hasher},
    marker::PhantomData,
    ptr,
    sync::Arc,
};

use arc_swap::{ArcSwap, AsRaw};
use siphasher::sip128::SipHasher13;

use crate::{inner::Inner, iter::Iter, snapshot::NodeRef};

mod error;
mod inner;
mod iter;
mod snapshot;

pub struct ConsistentHash<K, N, S = BuildHasherDefault<SipHasher13>, const R: usize = 4> {
    build_hash: S,
    inner: ArcSwap<Inner<N>>,
    _marker: PhantomData<fn() -> K>,
}

impl<K, N, S, const R: usize> ConsistentHash<K, N, S, R> {
    fn with_hasher(build_hash: S) -> Self {
        Self {
            build_hash,
            inner: Default::default(),
            _marker: PhantomData,
        }
    }

    pub fn contains_key(&self, hash: &u64) -> bool {
        self.inner.load().contains_key(hash)
    }

    pub fn is_empty(&self) -> bool {
        self.inner.load().is_empty()
    }

    pub fn virtual_nodes_len(&self) -> usize {
        self.inner.load().virtual_nodes_len()
    }

    pub fn nodes_len(&self) -> usize {
        self.inner.load().nodes_len()
    }
}

impl<K, N, S, const R: usize> ConsistentHash<K, N, S, R>
where
    K: Hash,
    S: BuildHasher,
{
    fn vnode_hash<Q>(&self, k: &Q, idx: usize) -> u64
    where
        K: Borrow<Q>,
        Q: Hash + ?Sized,
    {
        let mut hasher = self.build_hash.build_hasher();

        k.hash(&mut hasher);
        hasher.write(&(idx as u32).to_le_bytes());

        hasher.finish()
    }

    fn vnodes_hash<Q>(&self, k: &Q) -> Option<[u64; R]>
    where
        K: Borrow<Q>,
        Q: Hash + ?Sized,
    {
        let mut hashes = [0u64; R];
        for (idx, hash) in hashes.iter_mut().enumerate() {
            *hash = self.vnode_hash(k, idx);
        }

        hashes.sort_unstable();
        if hashes.array_windows().any(|[prev, next]| prev == next) {
            return None;
        }

        Some(hashes)
    }

    pub fn get_node(&self, hash: u64) -> Option<NodeRef<N>> {
        let inner = self.inner.load_full();
        let &(_, token) = inner.get_node(hash)?;

        Some(NodeRef::new(inner, token))
    }

    pub(crate) fn search_nearest_forward(&self, hash: u64) -> Option<(u64, NodeRef<N>)> {
        let inner = self.inner.load_full();
        let &(hash, token) = inner.nearest_forward_node_index(hash)?;

        Some((hash, NodeRef::new(inner, token)))
    }

    pub fn search_nearest(&self, hash: u64) -> Option<(u64, NodeRef<N>)> {
        let inner = self.inner.load_full();
        let &(hash, token) = inner.nearest_node_index(hash)?;

        Some((hash, NodeRef::new(inner, token)))
    }
}

impl<K, N, S, const R: usize> ConsistentHash<K, N, S, R>
where
    N: Clone,
    K: Hash,
    S: BuildHasher,
{
    pub fn insert<Q>(&self, k: &Q, node: N) -> Result<NodeRef<N>, error::Error<N>>
    where
        K: Borrow<Q>,
        Q: Hash + ?Sized,
    {
        let Some(hashes) = self.vnodes_hash(k) else {
            return Err(error::Error::VnodeHashCollision(node));
        };

        // TODO: https://github.com/vorner/arc-swap/pull/212
        let mut cur = self.inner.load();
        let node = loop {
            let new = Inner::clone(&cur);
            let (new, token) = new.insert_with(&hashes, node.clone())?;
            let inner = Arc::new(new);

            let prev = self.inner.compare_and_swap(&*cur, inner.clone());
            let swapped = ptr_eq(&*cur, &*prev);
            if swapped {
                break NodeRef::new(inner, token);
            } else {
                cur = prev;
            }
        };

        Ok(node)
    }

    pub fn remove<Q>(&self, k: &Q) -> Option<N>
    where
        K: Borrow<Q>,
        Q: Hash + ?Sized,
    {
        let hashes = self.vnodes_hash(k)?;

        // TODO: https://github.com/vorner/arc-swap/pull/212
        let mut cur = self.inner.load();
        let node = loop {
            let new = Inner::clone(&cur);
            let (new, node) = new.remove_with(&hashes)?;

            let prev = self.inner.compare_and_swap(&*cur, new.into());
            let swapped = ptr_eq(&*cur, &*prev);
            if swapped {
                break node;
            } else {
                cur = prev;
            }
        };

        Some(node)
    }

    pub fn get_node_cloned(&self, hash: u64) -> Option<N> {
        let inner = self.inner.load_full();
        let &(_, token) = inner.get_node(hash)?;

        Some(inner[token].clone())
    }

    pub fn search_nearest_cloned(&self, hash: u64) -> Option<(u64, N)> {
        let inner = self.inner.load_full();
        let &(hash, token) = inner.nearest_node_index(hash)?;

        Some((hash, inner[token].clone()))
    }

    pub fn iter(&self) -> Iter<'_, K, N, S, R> {
        self.into_iter()
    }
}

impl<K, V, S: Default, const R: usize> Default for ConsistentHash<K, V, S, R> {
    fn default() -> Self {
        Self::with_hasher(Default::default())
    }
}

impl<'a, K, N, S, const R: usize> IntoIterator for &'a ConsistentHash<K, N, S, R>
where
    N: Clone,
    K: Hash,
    S: BuildHasher,
{
    type Item = NodeRef<N>;

    type IntoIter = Iter<'a, K, N, S, R>;

    fn into_iter(self) -> Self::IntoIter {
        Iter {
            inner: self,
            head: 0,
            tail: u64::MAX,
        }
    }
}

// TODO: https://github.com/vorner/arc-swap/pull/212
fn ptr_eq<Base, A, B>(a: A, b: B) -> bool
where
    A: AsRaw<Base>,
    B: AsRaw<Base>,
{
    let a = a.as_raw();
    let b = b.as_raw();
    ptr::eq(a, b)
}

#[cfg(test)]
mod tests {
    use std::hash::Hasher;

    use super::*;

    #[test]
    fn test_search() {
        #[derive(Default)]
        struct Hash(i32);

        impl Hasher for Hash {
            fn finish(&self) -> u64 {
                self.0 as u64
            }

            fn write(&mut self, _bytes: &[u8]) {}

            fn write_i32(&mut self, i: i32) {
                self.0 = i
            }
        }

        let ring: ConsistentHash<i32, i32, BuildHasherDefault<Hash>, 1> =
            ConsistentHash::with_hasher(BuildHasherDefault::<Hash>::new());

        ring.insert(&1, 1).unwrap();
        ring.insert(&50, 50).unwrap();
        ring.insert(&100, 100).unwrap();

        assert_eq!(
            ring.search_nearest(0)
                .map(|(hash, node_ref)| (hash, *node_ref)),
            Some((1, 1))
        );
        assert_eq!(
            ring.search_nearest_forward(0)
                .map(|(hash, node_ref)| (hash, *node_ref)),
            Some((100, 100))
        );
        assert_eq!(
            ring.search_nearest(50)
                .map(|(hash, node_ref)| (hash, *node_ref)),
            Some((50, 50))
        );
        assert_eq!(
            ring.search_nearest_forward(50)
                .map(|(hash, node_ref)| (hash, *node_ref)),
            Some((1, 1))
        );
        assert_eq!(
            ring.search_nearest(100)
                .map(|(hash, node_ref)| (hash, *node_ref)),
            Some((100, 100))
        );
        assert_eq!(
            ring.search_nearest_forward(100)
                .map(|(hash, node_ref)| (hash, *node_ref)),
            Some((50, 50))
        );
        assert_eq!(
            ring.search_nearest(80)
                .map(|(hash, node_ref)| (hash, *node_ref)),
            Some((100, 100))
        );
        assert_eq!(
            ring.search_nearest_forward(80)
                .map(|(hash, node_ref)| (hash, *node_ref)),
            Some((50, 50))
        );
        assert_eq!(
            ring.search_nearest(150)
                .map(|(hash, node_ref)| (hash, *node_ref)),
            Some((1, 1))
        );
        assert_eq!(
            ring.search_nearest_forward(150)
                .map(|(hash, node_ref)| (hash, *node_ref)),
            Some((100, 100))
        );
    }

    #[test]
    fn test_replicate() {
        let ring: ConsistentHash<i32, i32, BuildHasherDefault<SipHasher13>, 4> =
            ConsistentHash::with_hasher(BuildHasherDefault::<SipHasher13>::new());

        assert!(ring.is_empty());
        ring.insert(&1, 1).unwrap();
        ring.insert(&50, 50).unwrap();
        ring.insert(&100, 100).unwrap();

        assert_eq!(ring.nodes_len(), 3);
        assert_eq!(ring.virtual_nodes_len(), 12);

        ring.remove(&1);
        assert_eq!(ring.nodes_len(), 2);
        assert_eq!(ring.virtual_nodes_len(), 8);

        ring.remove(&50);
        assert_eq!(ring.nodes_len(), 1);
        assert_eq!(ring.virtual_nodes_len(), 4);

        ring.remove(&100);
        assert_eq!(ring.nodes_len(), 0);
        assert_eq!(ring.virtual_nodes_len(), 0);
    }
}
