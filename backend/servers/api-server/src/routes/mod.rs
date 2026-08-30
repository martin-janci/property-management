//! Route modules for API server.
//!
//! Each module handles a specific domain and provides its own router.

pub mod admin;
pub mod admin_tenant_lifecycle;
pub mod admin_tenants;
pub mod agencies;
// Phase 1: Tenant Resolution — agency-create provisioning sliver.
// Merged into `platform_admin::router()`; not nested directly in `lib.rs`.
pub mod agency_provisioning;
pub mod ai;
// Phase 3: Hosting & Theming — Caddy on-demand TLS gate (internal-only).
pub mod caddy_ask;
// Phase 3: Hosting & Theming — public per-host tenant config endpoint.
pub mod accounting;
pub mod announcements;
pub mod auth;
pub mod buildings;
pub mod compliance;
pub mod critical_notifications;
pub mod delegations;
pub mod documents;
pub mod facilities;
pub mod faults;
pub mod financial;
pub mod gdpr;
pub mod granular_notifications;
pub mod health;
pub mod help;
pub mod insurance;
pub mod integrations;
pub mod iot;
pub mod leases;
pub mod listings;
pub mod messaging;
pub mod meters;
pub mod mfa;
pub mod my_units;
pub mod neighbors;
pub mod notification_preferences;
pub mod push_tokens;
pub mod rate_limit;
// Epic 8A, Story 8A.3 — WebSocket realtime notification sync
pub mod oauth;
pub mod onboarding;
pub mod organizations;
pub mod pagination;
pub mod person_months;
pub mod platform_admin;
pub mod rentals;
pub mod signatures;
pub mod templates;
pub mod tenant_config;
pub mod unit_residents;
pub mod vendors;
pub mod voting;
pub mod work_orders;
pub mod ws_notifications;

// Epic 23: Emergency Management
pub mod emergency;

// Epic 24: Budget & Financial Planning
pub mod budgets;

// Epic 25: Legal Document & Compliance
pub mod legal;

// Epic 26: Platform Subscription & Billing
pub mod subscriptions;

// Epic 30: Government Portal Integration
pub mod government_portal;

// Epic 37: Community & Social Features
pub mod community;

// Epic 38: Workflow Automation
pub mod automation;

// Epic 54: Forms Management
pub mod forms;

// Epic 55: Advanced Reporting & Analytics
pub mod reports;

// Epic 58: Package & Visitor Management
pub mod package_visitor;

// Epic 59: News & Media Management
pub mod news_articles;

// Epic 65: Energy & Sustainability Tracking
pub mod energy;

// Epic 64: Advanced AI & LLM Capabilities
pub mod registry;

// Epic 66: Platform Migration & Data Import
pub mod migration;

// Epic 67: Advanced Compliance (AML/DSA)
pub mod aml_dsa;

// Epic 68: Service Provider Marketplace
pub mod marketplace;

// Epic 70: Competitive Feature Enhancements — removed (PAP-33): dead scaffold, no migration, no product backing

// Epic 71: Cross-Cutting Infrastructure
pub mod infrastructure;

// Epic 72: Regional Legal Compliance (SK/CZ)
pub mod regional_compliance;

// Epic 73: Infrastructure & Operations
pub mod operations;

// Epic 74: Owner Investment Analytics
pub mod owner_analytics;

// Epic 77: Dispute Resolution
pub mod disputes;

// Epic 93: Voice Assistant & OAuth Completion
pub mod voice_webhooks;

// Epic 105: Portal Syndication
pub mod portal_webhooks;

// Epic 108: Feature Packages & Bundles
pub mod feature_packages;

// Epic 109: User Type Feature Experience
pub mod features;

// UC-12: Utility Outages
pub mod outages;

// Epic 132: Dynamic Rent Pricing & Market Analytics
pub mod market_pricing;

// Epic 133: AI Lease Abstraction & Document Intelligence
pub mod lease_abstraction;

// Epic 134: Predictive Maintenance & Equipment Intelligence
pub mod predictive_maintenance;

// Epic 135: Enhanced Tenant Screening with AI Risk Scoring
pub mod enhanced_tenant_screening;

// Epic 136: ESG Reporting Dashboard
pub mod esg_reporting;

// Epic 137: Smart Building Certification
pub mod building_certifications;

// Epic 138: Automated Property Valuation Model
pub mod property_valuation;

// Epic 139: Investor Portal & ROI Reporting
pub mod investor_portal;

// Epic 140: Multi-Property Portfolio Analytics
pub mod portfolio_analytics;

// Epic 141: Reserve Fund Management
pub mod reserve_funds;

// Epic 142: Violation Tracking & Enforcement
pub mod violations;

// Epic 143: Board Meeting Management
pub mod board_meetings;

// Epic 144: Portfolio Performance Analytics
pub mod portfolio_performance;

// Epic 145: Multi-Currency & Cross-Border Support
pub mod multi_currency;

// Epic 146: Enhanced Data Residency Controls
pub mod data_residency;

// Epic 150: API Ecosystem Expansion
pub mod api_ecosystem;

// Layout Control-Plane (Task 4)
pub mod layout;
