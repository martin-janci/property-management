//! Background services for reality-server.

pub mod saved_search_alerts;
pub mod search_alert_drainer;

pub use saved_search_alerts::{SavedSearchAlertConfig, SavedSearchAlertWorker};
pub use search_alert_drainer::{
    AlertEmailTransport, AlertNotification, AlertPushTransport, SearchAlertDrainerConfig,
    SearchAlertDrainerWorker,
};
