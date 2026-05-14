//! Public health surface.

pub mod types;
pub mod ids;
pub mod graph;
pub mod checker;
pub mod resolver;
pub mod provider;
pub mod platforms;

pub use types::*;
pub use graph::{DependencySpec, dependency_specs, spec_for};
pub use checker::{Checker, CheckOutcome};
pub use resolver::{Resolver, ResolveOutcome};
pub use provider::{PlatformProvider, detect_provider};
