use std::{cmp::Ordering, ops::Index};

use slab::Slab;

use crate::error;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[repr(transparent)]
pub struct Token(usize);

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub(crate) struct Inner<N> {
    ring: Box<[(u64, Token)]>,
    nodes: Slab<N>,
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

    pub(crate) fn get_node(&self, hash: u64) -> Option<&(u64, Token)> {
        let idx = self.ring.binary_search_by_key(&hash, |bin| bin.0).ok()?;

        self.ring.get(idx)
    }

    pub(crate) fn nearest_node_index(&self, hash: u64) -> Option<&(u64, Token)> {
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
    ) -> Result<(Self, Token), error::Error<N>> {
        if let Some(hash) = self.contains_keys(hashes) {
            return Err(error::Error::AlreadyExists(hash, node));
        }

        let token = Token(self.nodes.insert(node));

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
            .remove(token.expect("there must have at least one node").0);

        Some((
            Self {
                ring: ring.into_boxed_slice(),
                nodes: self.nodes,
            },
            node,
        ))
    }
}

impl<N> Index<usize> for Inner<N> {
    type Output = (u64, Token);

    fn index(&self, index: usize) -> &Self::Output {
        &self.ring[index]
    }
}

impl<N> Index<Token> for Inner<N> {
    type Output = N;

    fn index(&self, index: Token) -> &Self::Output {
        &self.nodes[index.0]
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
