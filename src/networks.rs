mod accumulator;
mod activation;
mod layer;
mod policy;
mod value;

pub use policy::{PolicyFileDefaultName, PolicyNetwork, UnquantisedPolicyNetwork};
pub use value::{UnquantisedValueNetwork, ValueFileDefaultName, ValueNetwork};

const QA: i16 = 512;
