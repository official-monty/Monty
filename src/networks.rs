mod accumulator;
mod layer;
mod policy;
mod value;

pub use policy::{PolicyFileDefaultName, PolicyNetwork, SubNet};
pub use value::{UnquantisedValueNetwork, ValueFileDefaultName, ValueNetwork};

const QA: i16 = 512;
