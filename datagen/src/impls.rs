#[cfg(feature = "ataxx")]
pub mod ataxx;

#[cfg(not(feature = "ataxx"))]
#[cfg(not(feature = "shatranj"))]
pub mod chess;

#[cfg(feature = "shatranj")]
pub mod shatranj;
