use std::{
    borrow::Borrow,
    collections::BTreeMap,
    hash::{BuildHasher, BuildHasherDefault, Hash},
    marker::PhantomData,
};

use siphasher::sip128::SipHasher13;

pub struct ConsistentHash<K, N, S = BuildHasherDefault<SipHasher13>> {
    build_hash: S,
    ring: BTreeMap<u64, N>,
    _marker: PhantomData<fn() -> K>,
}

impl<K, N, S> ConsistentHash<K, N, S> {
    fn with_hasher(build_hash: S) -> Self {
        Self {
            build_hash,
            ring: Default::default(),
            _marker: PhantomData,
        }
    }
}

impl<K, N, S> ConsistentHash<K, N, S>
where
    K: Hash,
    S: BuildHasher,
{
    // TODO: handle collisions
    pub fn insert<Q>(&mut self, k: &Q, node: N)
    where
        K: Borrow<Q>,
        Q: Hash + ?Sized,
    {
        let hash = self.build_hash.hash_one(k);

        self.ring.insert(hash, node);
    }

    pub fn remove<Q>(&mut self, k: &Q) -> Option<N>
    where
        K: Borrow<Q>,
        Q: Hash,
    {
        let hash = self.build_hash.hash_one(k);

        self.ring.remove(&hash)
    }

    pub fn search_nearest(&self, hash: u64) -> Option<(&u64, &N)> {
        self.ring
            .range(hash..)
            .next()
            .or_else(|| self.ring.range(..).next())
    }

    pub fn search_nearest_mut(&mut self, hash: u64) -> Option<(&u64, &mut N)> {
        match self.ring.range_mut(hash..).next() {
            x @ Some(_) => x,
            None => self.ring.range_mut(..).next(),
        }
    }
}

impl<K, V, S: Default> Default for ConsistentHash<K, V, S> {
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

        let mut ring: ConsistentHash<i32, i32, BuildHasherDefault<Hash>> =
            ConsistentHash::with_hasher(BuildHasherDefault::<Hash>::new());

        ring.insert(&1, 1);
        ring.insert(&50, 50);
        ring.insert(&100, 100);

        assert_eq!(ring.search_nearest(0), Some((&1, &1)));
        assert_eq!(ring.search_nearest(50), Some((&50, &50)));
        assert_eq!(ring.search_nearest(100), Some((&100, &100)));
        assert_eq!(ring.search_nearest(80), Some((&100, &100)));
        assert_eq!(ring.search_nearest(150), Some((&1, &1)));
    }
}
