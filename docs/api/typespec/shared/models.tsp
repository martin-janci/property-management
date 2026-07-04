namespace PropertyManagement.Shared;

@doc("Unique identifier (UUID v4)")
scalar uuid extends string;

@doc("ISO 8601 date-time string")
scalar dateTime extends string;

@doc("ISO 8601 date string (YYYY-MM-DD)")
scalar date extends string;

@doc("Email address")
@format("email")
scalar email extends string;

@doc("Phone number in E.164 format")
scalar phoneNumber extends string;

@doc("URL string")
@format("uri")
scalar urlString extends string;

// ---------------------------------------------------------------------------
// Closed enums for the multi-currency / cross-border domain (Epic 145).
//
// THREE SOURCES OF TRUTH — keep these member lists in sync:
//   1. Postgres enums  → backend/crates/db/migrations/00101_create_multi_currency.sql
//                        (`supported_currency`, `country_code`)
//   2. Rust enums      → backend/crates/db/src/models/multi_currency.rs
//                        (`SupportedCurrency`, `CountryCode`)
//   3. This TypeSpec   → drives OpenAPI + generated @ppt/api-client / Kotlin client
//
// Member names ARE the wire values: an unvalued TypeSpec enum member emits its
// name as the string value, so `EUR` -> "EUR", matching serde/sqlx
// `rename_all = "UPPERCASE"` (identity for these single-token acronyms).
// ---------------------------------------------------------------------------

@doc("Supported ISO 4217 currency codes for multi-currency operations")
enum SupportedCurrency {
  EUR, // Euro (base for most EU)
  CZK, // Czech Koruna
  CHF, // Swiss Franc
  GBP, // British Pound
  PLN, // Polish Zloty
  USD, // US Dollar
  HUF, // Hungarian Forint
  RON, // Romanian Leu
  BGN, // Bulgarian Lev
  HRK, // Croatian Kuna (legacy, now EUR)
  SEK, // Swedish Krona
  DKK, // Danish Krone
  NOK, // Norwegian Krone
}

@doc("Supported ISO 3166-1 alpha-2 country codes (EU/EEA focus)")
enum CountryCode {
  SK, // Slovakia
  CZ, // Czech Republic
  AT, // Austria
  DE, // Germany
  PL, // Poland
  HU, // Hungary
  CH, // Switzerland
  GB, // United Kingdom
  FR, // France
  IT, // Italy
  ES, // Spain
  NL, // Netherlands
  BE, // Belgium
  PT, // Portugal
  IE, // Ireland
  RO, // Romania
  BG, // Bulgaria
  HR, // Croatia
  SI, // Slovenia
  LU, // Luxembourg
  SE, // Sweden
  DK, // Denmark
  NO, // Norway
  FI, // Finland
}

@doc("Physical address")
model Address {
  @doc("Street address line 1")
  street1: string;

  @doc("Street address line 2 (optional)")
  street2?: string;

  @doc("City name")
  city: string;

  @doc("State or province")
  state?: string;

  @doc("Postal/ZIP code")
  postalCode: string;

  @doc("ISO 3166-1 alpha-2 country code")
  country: CountryCode;
}

@doc("Monetary amount with currency")
model Money {
  @doc("Amount in smallest currency unit (e.g., cents)")
  amount: int64;

  @doc("ISO 4217 currency code")
  currency: SupportedCurrency;
}

@doc("GPS coordinates")
model GeoLocation {
  @doc("Latitude (-90 to 90)")
  @minValue(-90)
  @maxValue(90)
  latitude: float64;

  @doc("Longitude (-180 to 180)")
  @minValue(-180)
  @maxValue(180)
  longitude: float64;
}

@doc("File attachment metadata")
model Attachment {
  @doc("Unique identifier")
  id: uuid;

  @doc("Original filename")
  filename: string;

  @doc("MIME type")
  mimeType: string;

  @doc("File size in bytes")
  sizeBytes: int64;

  @doc("Download URL")
  downloadUrl: urlString;

  @doc("Upload timestamp")
  uploadedAt: dateTime;
}

@doc("Base model for all entities with audit fields")
model AuditableEntity {
  @doc("Creation timestamp")
  createdAt: dateTime;

  @doc("Last update timestamp")
  updatedAt: dateTime;

  @doc("ID of user who created this entity")
  createdBy?: uuid;

  @doc("ID of user who last updated this entity")
  updatedBy?: uuid;
}

@doc("Localized content")
model LocalizedString {
  @doc("Default language value")
  default: string;

  @doc("Translations by ISO 639-1 language code")
  translations?: Record<string>;
}
