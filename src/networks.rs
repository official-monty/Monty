mod accumulator;
mod layer;
mod policy;
mod value;

pub use policy::{PolicyFileDefaultName, PolicyNetwork, SubNet};
pub use value::{ValueFileDefaultName, ValueNetwork, UnquantisedValueNetwork};

const QA: i16 = 255;
