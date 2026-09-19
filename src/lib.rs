use std::{
    borrow::Borrow,
    collections::BTreeMap,
    hash::{BuildHasher, BuildHasherDefault, Hash, Hasher},
    marker::PhantomData,
};

mod error;

use siphasher::sip128::SipHasher13;
use slab::Slab;

pub struct ConsistentHash<K, N, S = BuildHasherDefault<SipHasher13>, const R: usize = 4> {
    build_hash: S,
    ring: BTreeMap<u64, usize>,
    nodes: Slab<N>,
    _marker: PhantomData<fn() -> K>,
}

impl<K, N, S, const R: usize> ConsistentHash<K, N, S, R> {
    fn with_hasher(build_hash: S) -> Self {
        Self {
            build_hash,
            ring: Default::default(),
            nodes: Default::default(),
            _marker: PhantomData,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn len(&self) -> usize {
        self.ring.len()
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

    pub fn insert<Q>(&mut self, k: &Q, node: N) -> Result<&mut N, error::Error<N>>
    where
        K: Borrow<Q>,
        Q: Hash + ?Sized,
    {
        let mut hashes = [0u64; R];
        for (idx, hash) in hashes.iter_mut().enumerate() {
            let h = self.vnode_hash(k, idx);

            if self.ring.contains_key(&h) {
                return Err(error::Error::Collisions(h, node));
            }

            *hash = h;
        }

        let token = self.nodes.insert(node);

        for hash in hashes {
            self.ring.insert(hash, token);
        }

        Ok(&mut self.nodes[token])
    }

    pub fn remove<Q>(&mut self, k: &Q) -> Option<N>
    where
        K: Borrow<Q>,
        Q: Hash + ?Sized,
    {
        let mut hashes = [0u64; R];
        for (idx, slot) in hashes.iter_mut().enumerate() {
            let h = self.vnode_hash(k, idx);

            if self.ring.contains_key(&h) {
                *slot = h;
            } else {
                return None;
            }
        }

        let mut token = None;
        for hash in hashes {
            let Some(slot) = self.ring.remove(&hash) else {
                unreachable!("")
            };

            match token {
                Some(token) => assert_eq!(token, slot),
                None => token = Some(slot),
            }
        }

        Some(
            self.nodes
                .remove(token.expect("there must have at least one node")),
        )
    }

    fn nearest_node_index(&self, hash: u64) -> Option<(&u64, &usize)> {
        self.ring
            .range(hash..)
            .next()
            .or_else(|| self.ring.first_key_value())
    }

    pub fn search_nearest(&self, hash: u64) -> Option<(u64, &N)> {
        let (&hash, &token) = self.nearest_node_index(hash)?;

        Some((hash, &self.nodes[token]))
    }

    pub fn search_nearest_mut(&mut self, hash: u64) -> Option<(u64, &mut N)> {
        let (&hash, &token) = self.nearest_node_index(hash)?;

        Some((hash, &mut self.nodes[token]))
    }
}

impl<K, V, S: Default, const R: usize> Default for ConsistentHash<K, V, S, R> {
    fn default() -> Self {
        Self::with_hasher(Default::default())
    }
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

        let mut ring: ConsistentHash<i32, i32, BuildHasherDefault<Hash>, 1> =
            ConsistentHash::with_hasher(BuildHasherDefault::<Hash>::new());

        ring.insert(&1, 1).unwrap();
        ring.insert(&50, 50).unwrap();
        ring.insert(&100, 100).unwrap();

        assert_eq!(ring.search_nearest(0), Some((1, &1)));
        assert_eq!(ring.search_nearest(50), Some((50, &50)));
        assert_eq!(ring.search_nearest(100), Some((100, &100)));
        assert_eq!(ring.search_nearest(80), Some((100, &100)));
        assert_eq!(ring.search_nearest(150), Some((1, &1)));
    }

    #[test]
    fn test_replicate() {
        let mut ring: ConsistentHash<i32, i32, BuildHasherDefault<SipHasher13>, 4> =
            ConsistentHash::with_hasher(BuildHasherDefault::<SipHasher13>::new());

        assert!(ring.is_empty());
        ring.insert(&1, 1).unwrap();
        ring.insert(&50, 50).unwrap();
        ring.insert(&100, 100).unwrap();

        assert_eq!(ring.len(), 12);

        ring.remove(&1);
        assert_eq!(ring.len(), 8);
        ring.remove(&50);
        assert_eq!(ring.len(), 4);
    }
}
