#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error<N> {
    AlreadyExists(u64, N),
    VnodeHashCollision(N),
}
