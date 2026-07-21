pub mod server;
pub mod routes;
pub mod state;
pub mod events;
pub(crate) mod handlers;
pub(crate) mod util;
pub(crate) mod gateway_init;
pub mod gateway_embedded;
pub mod gateway_config_loader;
// On-demand HF model provisioning wiring. The module's plan/builder helpers are
// gated behind `embedded-llama-cpp` (only that build enables the coldboot +
// hf-download engine wings); the module is always declared so the feature-gated
// items resolve when the feature is on.
pub(crate) mod model_provisioning;

pub use server::start_server;
