//! Common data types.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use validator::Validate;

/// Physical address.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, Validate)]
pub struct Address {
    /// Street address line 1
    #[validate(length(min = 1, max = 255))]
    pub street1: String,

    /// Street address line 2 (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub street2: Option<String>,

    /// City name
    #[validate(length(min = 1, max = 100))]
    pub city: String,

    /// State or province
    #[serde(skip_serializing_if = "Option::is_none")]
    pub state: Option<String>,

    /// Postal/ZIP code
    #[validate(length(min = 1, max = 20))]
    pub postal_code: String,

    /// ISO 3166-1 alpha-2 country code
    #[validate(length(equal = 2))]
    pub country: String,
}

/// Monetary amount with currency.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, ToSchema)]
pub struct Money {
    /// Amount in smallest currency unit (e.g., cents)
    pub amount: i64,

    /// ISO 4217 currency code
    pub currency: Currency,
}

impl Money {
    pub fn new(amount: i64, currency: Currency) -> Self {
        Self { amount, currency }
    }

    pub fn eur(amount: i64) -> Self {
        Self::new(amount, Currency::EUR)
    }

    pub fn usd(amount: i64) -> Self {
        Self::new(amount, Currency::USD)
    }

    /// Get amount as floating point (for display).
    pub fn as_decimal(&self) -> f64 {
        self.amount as f64 / 100.0
    }
}

/// Currency codes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub enum Currency {
    EUR,
    USD,
    GBP,
    CZK,
    PLN,
    HUF,
}

impl std::fmt::Display for Currency {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Currency::EUR => write!(f, "EUR"),
            Currency::USD => write!(f, "USD"),
            Currency::GBP => write!(f, "GBP"),
            Currency::CZK => write!(f, "CZK"),
            Currency::PLN => write!(f, "PLN"),
            Currency::HUF => write!(f, "HUF"),
        }
    }
}

/// GPS coordinates.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, ToSchema)]
pub struct GeoLocation {
    /// Latitude (-90 to 90)
    pub latitude: f64,

    /// Longitude (-180 to 180)
    pub longitude: f64,
}

impl GeoLocation {
    pub fn new(latitude: f64, longitude: f64) -> Self {
        Self {
            latitude,
            longitude,
        }
    }
}

/// File attachment metadata.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Attachment {
    /// Unique identifier
    pub id: uuid::Uuid,

    /// Original filename
    pub filename: String,

    /// MIME type
    pub mime_type: String,

    /// File size in bytes
    pub size_bytes: i64,

    /// Download URL
    pub url: String,

    /// Upload timestamp
    pub uploaded_at: chrono::DateTime<chrono::Utc>,
}

/// Pagination query parameters.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, Validate)]
pub struct PaginationQuery {
    /// Page number (1-indexed)
    #[validate(range(min = 1))]
    #[serde(default = "default_page")]
    pub page: i32,

    /// Items per page
    #[validate(range(min = 1, max = 100))]
    #[serde(default = "default_limit")]
    pub limit: i32,

    /// Sort field
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sort_by: Option<String>,

    /// Sort direction
    #[serde(default)]
    pub sort_order: SortOrder,
}

fn default_page() -> i32 {
    1
}

fn default_limit() -> i32 {
    20
}

impl Default for PaginationQuery {
    fn default() -> Self {
        Self {
            page: 1,
            limit: 20,
            sort_by: None,
            sort_order: SortOrder::Asc,
        }
    }
}

impl PaginationQuery {
    pub fn offset(&self) -> i64 {
        ((self.page - 1) * self.limit) as i64
    }
}

/// Sort direction.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum SortOrder {
    #[default]
    Asc,
    Desc,
}

/// Pagination metadata.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PaginationMeta {
    /// Current page number
    pub page: i32,

    /// Items per page
    pub limit: i32,

    /// Total number of items
    pub total_items: i64,

    /// Total number of pages
    pub total_pages: i32,

    /// Has next page
    pub has_next: bool,

    /// Has previous page
    pub has_previous: bool,
}

impl PaginationMeta {
    pub fn new(page: i32, limit: i32, total_items: i64) -> Self {
        let total_pages = ((total_items as f64) / (limit as f64)).ceil() as i32;
        Self {
            page,
            limit,
            total_items,
            total_pages,
            has_next: page < total_pages,
            has_previous: page > 1,
        }
    }
}

/// Paginated response wrapper.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PaginatedResponse<T> {
    /// Array of items
    pub data: Vec<T>,

    /// Pagination metadata
    pub pagination: PaginationMeta,
}

impl<T> PaginatedResponse<T> {
    pub fn new(data: Vec<T>, page: i32, limit: i32, total_items: i64) -> Self {
        Self {
            data,
            pagination: PaginationMeta::new(page, limit, total_items),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use validator::Validate;

    fn valid_address() -> Address {
        Address {
            street1: "Hlavna 1".to_string(),
            street2: None,
            city: "Bratislava".to_string(),
            state: None,
            postal_code: "81101".to_string(),
            country: "SK".to_string(),
        }
    }

    #[test]
    fn address_valid_passes_validation() {
        assert!(valid_address().validate().is_ok());
    }

    #[test]
    fn address_empty_street_fails() {
        let mut addr = valid_address();
        addr.street1 = String::new();
        assert!(addr.validate().is_err());
    }

    #[test]
    fn address_oversized_street_fails() {
        let mut addr = valid_address();
        addr.street1 = "x".repeat(256);
        assert!(addr.validate().is_err());
    }

    #[test]
    fn address_empty_city_fails() {
        let mut addr = valid_address();
        addr.city = String::new();
        assert!(addr.validate().is_err());
    }

    #[test]
    fn address_oversized_city_fails() {
        let mut addr = valid_address();
        addr.city = "x".repeat(101);
        assert!(addr.validate().is_err());
    }

    #[test]
    fn address_empty_postal_fails() {
        let mut addr = valid_address();
        addr.postal_code = String::new();
        assert!(addr.validate().is_err());
    }

    #[test]
    fn address_country_must_be_two_chars() {
        let mut addr = valid_address();
        addr.country = "SVK".to_string();
        assert!(addr.validate().is_err());

        addr.country = "S".to_string();
        assert!(addr.validate().is_err());
    }

    #[test]
    fn address_serializes_optional_fields_skipped() {
        let addr = valid_address();
        let json = serde_json::to_string(&addr).unwrap();
        assert!(!json.contains("street2"));
        assert!(!json.contains("state"));
    }

    #[test]
    fn money_constructors_set_currency() {
        assert_eq!(Money::eur(100).currency, Currency::EUR);
        assert_eq!(Money::usd(250).currency, Currency::USD);
        assert_eq!(Money::new(500, Currency::GBP).currency, Currency::GBP);
        assert_eq!(Money::new(500, Currency::GBP).amount, 500);
    }

    #[test]
    fn money_as_decimal_divides_by_100() {
        assert!((Money::eur(12345).as_decimal() - 123.45).abs() < f64::EPSILON);
        assert!((Money::eur(0).as_decimal() - 0.0).abs() < f64::EPSILON);
        assert!((Money::eur(-500).as_decimal() - -5.0).abs() < f64::EPSILON);
    }

    #[test]
    fn currency_display_matches_iso_code() {
        assert_eq!(Currency::EUR.to_string(), "EUR");
        assert_eq!(Currency::USD.to_string(), "USD");
        assert_eq!(Currency::GBP.to_string(), "GBP");
        assert_eq!(Currency::CZK.to_string(), "CZK");
        assert_eq!(Currency::PLN.to_string(), "PLN");
        assert_eq!(Currency::HUF.to_string(), "HUF");
    }

    #[test]
    fn currency_equality() {
        assert_eq!(Currency::EUR, Currency::EUR);
        assert_ne!(Currency::EUR, Currency::USD);
    }

    #[test]
    fn geo_location_new_assigns_fields() {
        let g = GeoLocation::new(48.1486, 17.1077);
        assert!((g.latitude - 48.1486).abs() < f64::EPSILON);
        assert!((g.longitude - 17.1077).abs() < f64::EPSILON);
    }

    #[test]
    fn pagination_query_defaults() {
        let q = PaginationQuery::default();
        assert_eq!(q.page, 1);
        assert_eq!(q.limit, 20);
        assert!(q.sort_by.is_none());
        assert!(matches!(q.sort_order, SortOrder::Asc));
    }

    #[test]
    fn pagination_query_offset_first_page_is_zero() {
        let q = PaginationQuery { page: 1, limit: 20, sort_by: None, sort_order: SortOrder::Asc };
        assert_eq!(q.offset(), 0);
    }

    #[test]
    fn pagination_query_offset_second_page() {
        let q = PaginationQuery { page: 2, limit: 20, sort_by: None, sort_order: SortOrder::Asc };
        assert_eq!(q.offset(), 20);
    }

    #[test]
    fn pagination_query_offset_arbitrary_page() {
        let q = PaginationQuery { page: 5, limit: 25, sort_by: None, sort_order: SortOrder::Asc };
        assert_eq!(q.offset(), 100);
    }

    #[test]
    fn pagination_query_validates_min_page() {
        let q = PaginationQuery { page: 0, limit: 10, sort_by: None, sort_order: SortOrder::Asc };
        assert!(q.validate().is_err());
    }

    #[test]
    fn pagination_query_validates_limit_range() {
        let q_zero = PaginationQuery { page: 1, limit: 0, sort_by: None, sort_order: SortOrder::Asc };
        assert!(q_zero.validate().is_err());

        let q_over = PaginationQuery { page: 1, limit: 101, sort_by: None, sort_order: SortOrder::Asc };
        assert!(q_over.validate().is_err());

        let q_ok = PaginationQuery { page: 1, limit: 100, sort_by: None, sort_order: SortOrder::Asc };
        assert!(q_ok.validate().is_ok());
    }

    #[test]
    fn pagination_meta_computes_total_pages_exact() {
        let m = PaginationMeta::new(1, 10, 100);
        assert_eq!(m.total_pages, 10);
    }

    #[test]
    fn pagination_meta_rounds_up_partial_page() {
        let m = PaginationMeta::new(1, 10, 95);
        assert_eq!(m.total_pages, 10);
    }

    #[test]
    fn pagination_meta_zero_items_yields_zero_pages() {
        let m = PaginationMeta::new(1, 10, 0);
        assert_eq!(m.total_pages, 0);
        assert!(!m.has_next);
        assert!(!m.has_previous);
    }

    #[test]
    fn pagination_meta_first_page_flags() {
        let m = PaginationMeta::new(1, 10, 30);
        assert!(m.has_next);
        assert!(!m.has_previous);
    }

    #[test]
    fn pagination_meta_middle_page_flags() {
        let m = PaginationMeta::new(2, 10, 30);
        assert!(m.has_next);
        assert!(m.has_previous);
    }

    #[test]
    fn pagination_meta_last_page_flags() {
        let m = PaginationMeta::new(3, 10, 30);
        assert!(!m.has_next);
        assert!(m.has_previous);
    }

    #[test]
    fn paginated_response_wraps_data_and_meta() {
        let resp = PaginatedResponse::new(vec![1, 2, 3], 1, 10, 3);
        assert_eq!(resp.data, vec![1, 2, 3]);
        assert_eq!(resp.pagination.total_items, 3);
        assert_eq!(resp.pagination.total_pages, 1);
    }

    #[test]
    fn sort_order_serializes_lowercase() {
        let asc = serde_json::to_string(&SortOrder::Asc).unwrap();
        let desc = serde_json::to_string(&SortOrder::Desc).unwrap();
        assert_eq!(asc, "\"asc\"");
        assert_eq!(desc, "\"desc\"");
    }

    #[test]
    fn sort_order_default_is_asc() {
        let order = SortOrder::default();
        assert!(matches!(order, SortOrder::Asc));
    }
}
