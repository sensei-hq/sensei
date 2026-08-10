pub mod server;
pub mod routes;
pub mod state;
pub(crate) mod resilience;
pub mod events;
pub(crate) mod handlers;
pub(crate) mod util;
pub(crate) mod gateway_init;
pub mod gateway_embedded;
pub mod gateway_config_loader;
// On-demand model provisioning (sensei-owned; downloads via the Ollama registry,
// not the gateway's HF puller — gateway#5). The `ModelProvisioning` type compiles
// in every build (so `SharedState` can carry it unconditionally); it is only
// CONSTRUCTED + wired in an `embedded-llama-cpp` build (see `gateway_init`).
pub(crate) mod model_provisioning;

pub use server::start_server;
