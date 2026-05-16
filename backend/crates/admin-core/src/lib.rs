#![allow(clippy::doc_overindented_list_items)]
//! Phase 5 — Super-admin Control Plane.
//!
//! `admin-core` is the shared crate consumed by both `api-server` and
//! `reality-server` admin route trees. It implements the *single auditable
//! mechanism* called out in the brainstorming session (axes A6 + A7 +
//! leak #21):
//!
//! * Capability registry ([`Capability`], [`CapabilityRegistry`]).
//! * MFA-gated authorization extractor ([`RequireCapability`]).
//! * Audit writer that hashes payloads before persistence ([`AuditWriter`]).
//! * Per-tenant settings store ([`SettingsStore`]).
//! * Short-lived impersonation tokens ([`ImpersonationService`]).
//!
//! The load-bearing pieces — the **append-only audit log** (DB trigger in
//! migration 00138) and the **MFA recency gate** — are what make the
//! "single catastrophic point" of leak #21 survivable.

pub mod audit;
pub mod capability;
pub mod error;
pub mod extractor;
pub mod impersonation;
pub mod mfa;
pub mod settings_store;

pub use audit::{AuditOutcome, AuditWriter, NoopAuditWriter, PgAuditWriter, RecordedAuditEvent};
pub use capability::{
    Capability, CapabilityGrant, CapabilityGrantsRepository, CapabilityRegistry,
    PgCapabilityGrantsRepository,
};
pub use error::{AdminError, MfaRequired};
pub use extractor::{require_capability, AdminDeps, RequireCapability};
pub use impersonation::{
    ImpersonationService, ImpersonationToken, IssuedImpersonationToken, PgImpersonationService,
    IMPERSONATION_TTL,
};
pub use mfa::{MfaRecency, NoopMfaRecency, PgMfaRecency, RECENT_MFA_WINDOW};
pub use settings_store::{PgSettingsStore, SettingsRecord, SettingsStore};
