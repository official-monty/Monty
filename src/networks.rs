mod accumulator;
mod activation;
mod layer;
mod policy;
mod value;

pub use policy::{PolicyFileDefaultName, PolicyNetwork, UnquantisedPolicyNetwork};
pub use value::{ValueFileDefaultName, ValueNetwork, UnquantisedValueNetwork};

const QA: i16 = 512;
