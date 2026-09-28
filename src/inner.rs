use std::cmp::Ordering;

use slab::Slab;

use crate::error;

#[derive(Clone)]
pub(crate) struct Inner<N> {
    pub(crate) ring: Box<[(u64, usize)]>,
    pub(crate) nodes: Slab<N>,
}

impl<N> Inner<N> {
    pub fn contains_key(&self, hash: &u64) -> bool {
        self.ring.binary_search_by_key(hash, |bin| bin.0).is_ok()
    }

    /// requires `hash` to be sorted
    fn contains_keys(&self, hashes: &[u64]) -> Option<u64> {
        let mut r = 0;
        let mut h = 0;

        while r < self.ring.len() && h < hashes.len() {
            match self.ring[r].0.cmp(&hashes[h]) {
                Ordering::Less => r += 1,
                Ordering::Greater => h += 1,
                Ordering::Equal => return Some(hashes[h]),
            }
        }

        None
    }

    /// requires `hash` to be sorted
    fn contains_all_keys(&self, hashes: &[u64]) -> bool {
        let mut r = 0;
        let mut h = 0;

        while r < self.ring.len() && h < hashes.len() {
            match self.ring[r].0.cmp(&hashes[h]) {
                Ordering::Less => r += 1,
                Ordering::Greater => return false,
                Ordering::Equal => {
                    r += 1;
                    h += 1;
                }
            }
        }

        h == hashes.len()
    }

    pub(crate) fn nearest_node_index(&self, hash: u64) -> Option<&(u64, usize)> {
        let idx = match self.ring.binary_search_by_key(&hash, |bin| bin.0) {
            Err(n) if n == self.ring.len() => 0,
            Ok(n) | Err(n) => n,
        };

        self.ring.get(idx)
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.ring.is_empty()
    }

    pub(crate) fn virtual_nodes_len(&self) -> usize {
        self.ring.len()
    }

    pub(crate) fn nodes_len(&self) -> usize {
        self.nodes.len()
    }
}

impl<N> Inner<N>
where
    N: Clone,
{
    pub(crate) fn insert_with(
        mut self,
        hashes: &[u64],
        node: N,
    ) -> Result<(Self, usize), error::Error<N>> {
        if let Some(hash) = self.contains_keys(hashes) {
            return Err(error::Error::AlreadyExists(hash, node));
        }

        let token = self.nodes.insert(node);

        let mut ring = self.ring.into_vec();
        ring.extend(hashes.iter().copied().zip(std::iter::repeat(token)));

        ring.sort_unstable_by_key(|bin| bin.0);

        Ok((
            Self {
                ring: ring.into_boxed_slice(),
                nodes: self.nodes,
            },
            token,
        ))
    }

    pub(crate) fn remove_with(mut self, hashes: &[u64]) -> Option<(Self, N)> {
        if !self.contains_all_keys(hashes) {
            return None;
        }

        let mut token = None;
        let mut ring = self.ring.into_vec();
        ring.retain(|bin| {
            if hashes.contains(&bin.0) {
                match token {
                    Some(token) => assert_eq!(token, bin.1),
                    None => token = Some(bin.1),
                }
                false
            } else {
                true
            }
        });

        let node = self
            .nodes
            .remove(token.expect("there must have at least one node"));

        Some((
            Self {
                ring: ring.into_boxed_slice(),
                nodes: self.nodes,
            },
            node,
        ))
    }
}

impl<N> Default for Inner<N> {
    fn default() -> Self {
        Self {
            ring: Default::default(),
            nodes: Default::default(),
        }
    }
}
