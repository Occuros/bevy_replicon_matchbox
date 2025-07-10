#[cfg(feature = "client")]
mod client;
#[cfg(feature = "server")]
mod server;
#[cfg(any(feature = "client", feature = "server"))]
pub mod shared;

#[cfg(feature = "client")]
pub use client::*;
#[cfg(feature = "server")]
pub use server::*;

#[cfg(any(feature = "client", feature = "server"))]
pub use shared::RepliconMatchboxPlugins;
