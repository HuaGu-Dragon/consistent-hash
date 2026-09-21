use std::collections::BTreeMap;

use slab::Slab;

use crate::error;

#[derive(Clone)]
pub(crate) struct Inner<N> {
    // TODO: replace with Box<[u64, usize]>
    pub(crate) ring: BTreeMap<u64, usize>,
    pub(crate) nodes: Slab<N>,
}

impl<N> Inner<N> {
    pub(crate) fn nearest_node_index(&self, hash: u64) -> Option<(&u64, &usize)> {
        self.ring
            .range(hash..)
            .next()
            .or_else(|| self.ring.first_key_value())
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.ring.is_empty()
    }

    pub(crate) fn len(&self) -> usize {
        self.ring.len()
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
        for hash in hashes.iter() {
            if self.ring.contains_key(hash) {
                return Err(error::Error::AlreadyExists(*hash, node));
            }
        }

        let token = self.nodes.insert(node);

        self.ring
            .extend(hashes.iter().copied().zip(std::iter::repeat(token)));

        Ok((
            Self {
                ring: self.ring,
                nodes: self.nodes,
            },
            token,
        ))
    }

    pub(crate) fn remove_with(mut self, hashes: &[u64]) -> Option<(Self, N)> {
        for hash in hashes.iter() {
            if !self.ring.contains_key(hash) {
                return None;
            }
        }

        let mut token = None;
        for hash in hashes {
            let Some(slot) = self.ring.remove(hash) else {
                unreachable!("")
            };

            match token {
                Some(token) => assert_eq!(token, slot),
                None => token = Some(slot),
            }
        }

        let node = self
            .nodes
            .remove(token.expect("there must have at least one node"));

        Some((
            Self {
                ring: self.ring,
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
