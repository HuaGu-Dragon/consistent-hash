#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error<N> {
    Collisions(u64, N),
}
