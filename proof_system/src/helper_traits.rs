use crypto::{Hasher, RandomCoin};

pub trait Options {
    type Field;
    type Hasher: Hasher;
}

/// This trait is supposed to include instance specifications
/// For example, in the case of a STARK instance, this would be
/// the dimentions of the trace.
pub trait Context {
    type Options;
}
pub trait Channel {
    type Hasher: Hasher;

}
