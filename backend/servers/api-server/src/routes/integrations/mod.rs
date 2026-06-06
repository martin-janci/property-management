//! External Integrations routes — split by surface.
//!
//! | Surface | Module | Routes |
//! |---------|--------|--------|
//! | install | `install` | connect/disconnect for Airbnb, Booking.com, portals |
//! | oauth   | `oauth`   | OAuth callback flows (Airbnb OAuth callback) |
//! | sync    | `sync`    | calendar, accounting, e-signature, video CRUD + sync |
//! | webhook | `webhook` | webhook subscriptions CRUD + incoming webhook receivers |
//! | booking_channel | `booking_channel` | Booking.com listing push + conflict detection (Gap 83-2) |
//! | token_rotation | `token_rotation` | Airbnb OAuth token refresh, rotation, revocation (Gap 83-1) |

pub mod booking_channel;
pub mod install;
pub mod oauth;
pub mod oauth_state;
pub mod sync;
pub mod token_rotation;
pub mod webhook;

use axum::Router;

use crate::state::AppState;

/// Assemble the integrations router from all surface sub-modules.
pub fn router() -> Router<AppState> {
    Router::new()
        .merge(sync::router())
        .merge(install::router())
        .merge(oauth::router())
        .merge(webhook::router())
        .merge(booking_channel::router())
        .merge(token_rotation::router())
}
