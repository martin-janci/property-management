/**
 * OAuth User Grants Types
 *
 * Types for the OAuth user grants management endpoints (Epic 10A, Story 10A-3).
 * Mirrors the `UserGrantWithClient` Rust struct from the backend.
 */

/**
 * A user's active OAuth grant, enriched with client metadata for display.
 *
 * Corresponds to `UserGrantWithClient` in `db/models/oauth.rs`.
 */
export interface UserGrant {
  /** Stable row id for the grant record. */
  id: string;
  /** OAuth client identifier string. */
  clientId: string;
  /** Human-readable application name. */
  clientName: string;
  /** Optional developer-supplied description. */
  clientDescription: string | null;
  /** Scopes the user granted to this application. */
  scopes: string[];
  /** ISO-8601 timestamp of when the user first approved this client. */
  grantedAt: string;
}
