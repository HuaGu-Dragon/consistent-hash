use std::{iter::FusedIterator, sync::Arc};

use crate::{inner::Inner, snapshot::NodeRef};

pub struct Iter<N> {
    snapshot: Arc<Inner<N>>,
    front: usize,
    back: usize,
}

impl<N> Iter<N> {
    pub(crate) fn new(snapshot: Arc<Inner<N>>) -> Self {
        let back = snapshot.virtual_nodes_len();
        Self {
            snapshot,
            front: 0,
            back,
        }
    }
}

impl<N> Iterator for Iter<N> {
    type Item = NodeRef<N>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.front >= self.back {
            return None;
        }

        let &(_, token) = &self.snapshot[self.front];
        self.front += 1;

        Some(NodeRef::new(Arc::clone(&self.snapshot), token))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.back - self.front;
        (remaining, Some(remaining))
    }
}

impl<N> DoubleEndedIterator for Iter<N> {
    fn next_back(&mut self) -> Option<Self::Item> {
        if self.front >= self.back {
            return None;
        }

        self.back -= 1;
        let &(_, token) = &self.snapshot[self.back];

        Some(NodeRef::new(Arc::clone(&self.snapshot), token))
    }
}

impl<N> ExactSizeIterator for Iter<N> {}
impl<N> FusedIterator for Iter<N> {}

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
