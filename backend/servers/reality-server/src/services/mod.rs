//! Background services for reality-server.

pub mod favorite_alerts;
pub mod saved_search_alerts;
pub mod search_alert_drainer;

pub use favorite_alerts::{FavoriteAlertConfig, FavoriteAlertWorker};
pub use saved_search_alerts::{SavedSearchAlertConfig, SavedSearchAlertWorker};
pub use search_alert_drainer::{
    AlertEmailTransport, AlertNotification, AlertPushTransport, SearchAlertDrainerConfig,
    SearchAlertDrainerWorker,
};
