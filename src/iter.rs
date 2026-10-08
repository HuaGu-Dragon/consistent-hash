use std::hash::{BuildHasher, Hash};

use crate::{ConsistentHash, snapshot::NodeRef};

pub struct Iter<'a, K, N, S, const R: usize> {
    pub(crate) inner: &'a ConsistentHash<K, N, S, R>,
    pub(crate) head: u64,
    pub(crate) tail: u64,
}

impl<'a, K, N, S, const R: usize> Iterator for Iter<'a, K, N, S, R>
where
    K: Hash,
    S: BuildHasher,
{
    type Item = NodeRef<N>;

    fn next(&mut self) -> Option<Self::Item> {
        let (hash, node) = self.inner.search_nearest(self.head)?;
        if hash < self.head || hash > self.tail {
            return None;
        }
        self.head = hash + 1;

        Some(node)
    }
}

impl<'a, K, N, S, const R: usize> DoubleEndedIterator for Iter<'a, K, N, S, R>
where
    K: Hash,
    S: BuildHasher,
{
    fn next_back(&mut self) -> Option<Self::Item> {
        let (hash, node) = self.inner.search_nearest_forward(self.tail)?;
        if hash > self.tail || hash < self.head {
            return None;
        }
        self.tail = hash - 1;

        Some(node)
    }
}

#[cfg(test)]
mod tests {
    use std::hash::{BuildHasherDefault, Hasher};

    use siphasher::sip::SipHasher13;

    use crate::ConsistentHash;

    #[test]
    fn test_iter() {
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

        let mut iter = ring.iter();

        assert_eq!(*iter.next().unwrap(), 1);
        assert_eq!(*iter.next().unwrap(), 50);
        assert_eq!(*iter.next().unwrap(), 100);
        assert!(iter.next().is_none());
    }

    #[test]
    fn test_double_end() {
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

        let mut iter = ring.iter();

        assert_eq!(*iter.next().unwrap(), 1);
        assert_eq!(*iter.next_back().unwrap(), 100);
        assert_eq!(*iter.next().unwrap(), 50);
        assert!(iter.next().is_none());
        assert!(iter.next_back().is_none());
    }

    #[test]
    fn test_iter_replicate() {
        let ring: ConsistentHash<i32, i32, BuildHasherDefault<SipHasher13>, 4> =
            ConsistentHash::with_hasher(BuildHasherDefault::<SipHasher13>::new());

        ring.insert(&1, 1).unwrap();
        ring.insert(&50, 50).unwrap();
        ring.insert(&100, 100).unwrap();

        let iter = ring.iter();

        assert_eq!(iter.count(), 12)
    }
}
