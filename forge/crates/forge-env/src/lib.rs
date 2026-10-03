//! # forge-env
//!
//! Docker-based environment for the Forge SWE-agent.
//!
//! Provides:
//! - [`DockerContainer`]: Docker container lifecycle management.
//! - [`BashSession`]: Persistent bash session inside a container.
//! - [`RepoConfig`]: Git repository operations (reset, patch, diff).
//! - [`SweEnvironment`]: High-level API combining all of the above.

pub mod bash_session;
pub mod docker;
pub mod e2b;
pub mod environment;
pub mod repo;
pub(crate) mod utils;

pub use bash_session::{BashSession, CommandOutput};
pub use docker::DockerContainer;
pub use environment::{EnvironmentConfig, SweEnvironment};
pub use repo::RepoConfig;
