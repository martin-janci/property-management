// backend/servers/deploy-server/src/infra/mod.rs
pub mod audit;
pub mod blue_green;
pub mod caddy;
pub mod docker;
pub mod gh;
pub mod git;
pub mod health;
pub mod postgres;
pub mod store;
pub mod traffic;

pub use audit::{auth_and_audit, AuthState, CallerIdentity};
pub use blue_green::{BlueGreenDeployer, BlueGreenSpec, StagingDeploySpec, StagingDeployer};
pub use caddy::CaddyClient;
pub use docker::{BackendDedicatedSpec, DockerClient, FrontendDevSpec};
pub use gh::{GhClient, WorkflowRun};
pub use git::GitFetcher;
pub use health::HealthProbe;
pub use postgres::PostgresOps;
pub use store::Store;
