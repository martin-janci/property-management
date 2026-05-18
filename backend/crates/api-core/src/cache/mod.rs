//! Tenant-aware cache wrappers (Phase 5.5 — Tenant Lifecycle & Operability).
//!
//! Direct use of the underlying `redis::Client` (or `integrations::RedisClient`
//! outside of [`tenanted_redis::TenantedRedis`]) is forbidden by the
//! `backend/scripts/lints/no-raw-redis.sh` CI gate. This is the defense for
//! brainstorming leak #20 ("Shared Redis cross-tenant"): un-namespaced keys
//! collide silently across tenants.

pub mod tenanted_redis;

pub use tenanted_redis::{TenantedRedis, TenantedRedisError};
