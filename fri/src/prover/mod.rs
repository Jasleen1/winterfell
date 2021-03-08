mod monolith;
pub use monolith::FriProver;

mod channel;
pub use channel::{DefaultProverChannel, ProverChannel};

#[cfg(test)]
mod distributed;

#[cfg(test)]
mod tests;
