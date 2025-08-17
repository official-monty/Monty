mod accumulator;
mod activation;
mod layer;
mod policy;
mod threats;
mod value;

pub use accumulator::Accumulator;

// Choose the file name type based on the feature
#[cfg(feature = "datagen")]
pub use policy::DatagenPolicyFileName as PolicyFileDefaultName;

#[cfg(not(feature = "datagen"))]
pub use policy::PolicyFileDefaultName;

pub use policy::{PolicyNetwork, UnquantisedPolicyNetwork, L1 as POLICY_L1};
pub use value::{ValueFileDefaultName, ValueNetwork};
