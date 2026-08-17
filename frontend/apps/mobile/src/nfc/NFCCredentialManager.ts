/**
 * NFC credential management for building access.
 *
 * Epic 49 - Story 49.4: NFC Building Access
 */
import AsyncStorage from '@react-native-async-storage/async-storage';
import * as SecureStore from 'expo-secure-store';

import type {
  AccessDenialReason,
  AccessLogEntry,
  GuestAccessInvitation,
  NFCCredential,
} from './types';

// NFC credentials are sensitive (they encode building access). They're stored
// in expo-secure-store (Android EncryptedSharedPreferences / iOS Keychain) so
// they're encrypted at rest. Access logs are not sensitive and remain in
// AsyncStorage for simpler querying of the last N entries.
//
// SecureStore has a per-value size cap of ~2 KB on Android: writing a larger
// value logs a warning and the write can silently fail, which would drop
// credentials we just fetched from the server. A single NFCCredential carries
// an `encryptedData` blob plus an `accessPoints` array, so even a handful of
// buildings can push the serialized list past 2 KB. We therefore split the
// serialized array across as many chunk slots as needed (each kept under the
// cap) and record the chunk count in a small manifest stored at
// CREDENTIALS_KEY. Reads reassemble the chunks; the legacy single-slot format
// (a raw JSON array under CREDENTIALS_KEY) is still read transparently.
const CREDENTIALS_KEY = 'ppt_nfc_credentials';
// Prefix for the per-chunk slots, e.g. `ppt_nfc_credentials_c0`.
const CREDENTIALS_CHUNK_PREFIX = 'ppt_nfc_credentials_c';
// Byte budget per chunk. SecureStore's Android cap is ~2048 bytes; leave
// headroom for encryption/framing overhead.
const MAX_CHUNK_BYTES = 1800;
// Upper bound on chunk slots we ever scan when cleaning up a corrupted/stale
// write. 64 * 1800 B ≈ 115 KB, far beyond the "tens of entries" we expect.
const MAX_CREDENTIAL_CHUNKS = 64;
// Legacy key used by earlier versions that stored credentials in AsyncStorage;
// read-through once at startup so upgraded users don't lose their credentials.
const LEGACY_CREDENTIALS_KEY = '@ppt/nfc_credentials';
const ACCESS_LOG_KEY = '@ppt/access_log';
const MAX_LOG_ENTRIES = 100;

/** Manifest written at CREDENTIALS_KEY describing the chunked credential blob. */
interface CredentialManifest {
  v: 1;
  chunks: number;
}

/** UTF-8 byte length of a single Unicode code point. */
function codePointUtf8Bytes(codePoint: number): number {
  if (codePoint < 0x80) return 1;
  if (codePoint < 0x800) return 2;
  if (codePoint < 0x10000) return 3;
  return 4;
}

/**
 * Split a string into chunks whose UTF-8 encodings each stay within `maxBytes`.
 * Iterates by code point (surrogate-safe), so `chunks.join('')` reproduces the
 * original string exactly. Always returns at least one chunk (possibly empty).
 */
function chunkByUtf8Bytes(str: string, maxBytes: number): string[] {
  const chunks: string[] = [];
  let current = '';
  let currentBytes = 0;
  for (const ch of str) {
    const chBytes = codePointUtf8Bytes(ch.codePointAt(0) ?? 0);
    if (currentBytes + chBytes > maxBytes && current.length > 0) {
      chunks.push(current);
      current = '';
      currentBytes = 0;
    }
    current += ch;
    currentBytes += chBytes;
  }
  if (current.length > 0 || chunks.length === 0) {
    chunks.push(current);
  }
  return chunks;
}

/**
 * Manages NFC credentials for building access.
 */
export class NFCCredentialManager {
  private apiBaseUrl: string;
  private authToken: string | null = null;
  private credentials: NFCCredential[] = [];

  constructor(apiBaseUrl: string) {
    this.apiBaseUrl = apiBaseUrl;
  }

  /**
   * Set authentication token.
   */
  setAuthToken(token: string | null): void {
    this.authToken = token;
  }

  /**
   * Initialize and load stored credentials.
   */
  async initialize(): Promise<void> {
    await this.loadStoredCredentials();
  }

  /**
   * Fetch credentials from server.
   */
  async fetchCredentials(): Promise<NFCCredential[]> {
    try {
      const response = await this.apiRequest<{ credentials: NFCCredential[] }>(
        '/api/v1/access/credentials'
      );

      this.credentials = response.credentials;
      await this.storeCredentials();

      return this.credentials;
    } catch (error) {
      // Fall back to cached credentials
      console.warn('Failed to fetch credentials, using cached:', error);
      return this.credentials;
    }
  }

  /**
   * Get credential for a specific building.
   */
  getCredentialForBuilding(buildingId: string): NFCCredential | undefined {
    return this.credentials.find((c) => c.buildingId === buildingId && c.status === 'active');
  }

  /**
   * Get all active credentials.
   */
  getActiveCredentials(): NFCCredential[] {
    const nowMs = Date.now();
    return this.credentials.filter(
      (c) => c.status === 'active' && new Date(c.validUntil).getTime() > nowMs
    );
  }

  /**
   * Get credential by ID.
   */
  getCredentialById(id: string): NFCCredential | undefined {
    return this.credentials.find((c) => c.id === id);
  }

  /**
   * Refresh a credential (get new encrypted data).
   */
  async refreshCredential(credentialId: string): Promise<NFCCredential> {
    const response = await this.apiRequest<{ credential: NFCCredential }>(
      `/api/v1/access/credentials/${credentialId}/refresh`,
      'POST'
    );

    // Update local copy
    const index = this.credentials.findIndex((c) => c.id === credentialId);
    if (index >= 0) {
      this.credentials[index] = response.credential;
      await this.storeCredentials();
    }

    return response.credential;
  }

  /**
   * Log an access attempt.
   */
  async logAccessAttempt(
    credentialId: string,
    accessPointId: string,
    accessPointName: string,
    result: 'granted' | 'denied',
    denialReason?: AccessDenialReason
  ): Promise<void> {
    const entry: AccessLogEntry = {
      id: `log-${Date.now()}-${Math.random().toString(36).substring(2, 9)}`,
      credentialId,
      buildingId: this.getCredentialById(credentialId)?.buildingId ?? '',
      accessPointId,
      accessPointName,
      result,
      denialReason: denialReason as AccessLogEntry['denialReason'] | undefined,
      timestamp: new Date().toISOString(),
    };

    // Store locally
    const log = await this.getAccessLog();
    log.unshift(entry);
    if (log.length > MAX_LOG_ENTRIES) {
      log.pop();
    }
    await AsyncStorage.setItem(ACCESS_LOG_KEY, JSON.stringify(log));

    // Sync to server
    try {
      await this.apiRequest('/api/v1/access/log', 'POST', entry);
    } catch {
      // Will sync later
    }
  }

  /**
   * Get access log.
   */
  async getAccessLog(): Promise<AccessLogEntry[]> {
    // The stored blob may be corrupted (partial writes, manual app-data
    // restore, OS-level storage wipe). Never let a parse failure throw:
    // logAccessAttempt() calls this AFTER access has already been granted at
    // the door, so a corrupted blob must not reject the tap flow. Fall back to
    // an empty log rather than propagating the SyntaxError.
    const stored = await AsyncStorage.getItem(ACCESS_LOG_KEY);
    if (!stored) {
      return [];
    }
    try {
      return JSON.parse(stored);
    } catch (err) {
      console.warn('[nfc] Stored access log corrupted; treating as empty.', err);
      return [];
    }
  }

  /**
   * Get recent access attempts.
   */
  async getRecentAccessAttempts(limit = 10): Promise<AccessLogEntry[]> {
    const log = await this.getAccessLog();
    return log.slice(0, limit);
  }

  /**
   * Create a guest access invitation.
   */
  async createGuestInvitation(
    buildingId: string,
    guestName: string,
    guestEmail: string,
    accessPoints: string[],
    validFrom: string,
    validUntil: string,
    maxEntries?: number
  ): Promise<GuestAccessInvitation> {
    const response = await this.apiRequest<{ invitation: GuestAccessInvitation }>(
      '/api/v1/access/invitations',
      'POST',
      {
        buildingId,
        guestName,
        guestEmail,
        accessPoints,
        validFrom,
        validUntil,
        maxEntries,
      }
    );

    return response.invitation;
  }

  /**
   * Get guest invitations for a building.
   */
  async getGuestInvitations(buildingId: string): Promise<GuestAccessInvitation[]> {
    const response = await this.apiRequest<{ invitations: GuestAccessInvitation[] }>(
      `/api/v1/access/invitations?building_id=${buildingId}`
    );

    return response.invitations;
  }

  /**
   * Cancel a guest invitation.
   */
  async cancelGuestInvitation(invitationId: string): Promise<void> {
    await this.apiRequest(`/api/v1/access/invitations/${invitationId}/cancel`, 'POST');
  }

  /**
   * Report lost phone / emergency revoke all credentials.
   *
   * The local credential cache is cleared BEFORE the server call and always,
   * even when the server revoke fails. A failed (or unreachable) server revoke
   * must never leave credentials usable offline — that would defeat the entire
   * point of the "lost phone" panic action, since the encrypted credentials in
   * SecureStore can be presented at an access point without any network. We
   * therefore clear locally first (blocking offline use immediately), then
   * surface any API failure to the caller so it can prompt a retry once the
   * device is back online.
   */
  async emergencyRevokeAll(): Promise<void> {
    // Clear local cache first so a subsequent API failure cannot leave
    // credentials usable offline.
    this.credentials = [];
    await this.storeCredentials();

    // Now attempt the server-side revoke. If it fails, the local cache is
    // already cleared; re-throw so the failure surfaces to the caller.
    await this.apiRequest('/api/v1/access/credentials/revoke-all', 'POST');
  }

  /**
   * Check if credentials need refresh.
   */
  credentialsNeedRefresh(): boolean {
    const refreshThreshold = 24 * 60 * 60 * 1000; // 24 hours
    const now = Date.now();

    for (const credential of this.credentials) {
      if (credential.status !== 'active') continue;

      const validUntil = new Date(credential.validUntil).getTime();
      if (validUntil - now < refreshThreshold) {
        return true;
      }
    }

    return false;
  }

  /**
   * Update credential usage.
   */
  async updateCredentialUsage(credentialId: string): Promise<void> {
    const credential = this.credentials.find((c) => c.id === credentialId);
    if (credential) {
      credential.lastUsed = new Date().toISOString();
      credential.usageCount += 1;
      await this.storeCredentials();
    }
  }

  // Private methods

  private async loadStoredCredentials(): Promise<void> {
    // Stored credentials may be corrupted (partial writes, manual app-data
    // restore, OS wipe of secure store) or split across chunk slots (large
    // credential sets, see storeCredentials). Never let a read/parse failure
    // crash app startup: fall back to an empty list and drop the bad blob.
    const stored = await SecureStore.getItemAsync(CREDENTIALS_KEY);
    if (stored) {
      try {
        this.credentials = await this.deserializeStoredCredentials(stored);
        return;
      } catch (err) {
        console.warn('[nfc] Stored credentials corrupted; discarding.', err);
        this.credentials = [];
        await this.clearStoredCredentials();
        return;
      }
    }
    // One-time migration from the old AsyncStorage location.
    const legacy = await AsyncStorage.getItem(LEGACY_CREDENTIALS_KEY);
    if (legacy) {
      try {
        this.credentials = JSON.parse(legacy);
        await this.storeCredentials();
      } catch (err) {
        console.warn('[nfc] Legacy credentials corrupted; discarding.', err);
        this.credentials = [];
      }
      await AsyncStorage.removeItem(LEGACY_CREDENTIALS_KEY);
    }
  }

  /**
   * Reconstruct the credential array from what's stored at CREDENTIALS_KEY.
   * Handles both the chunked manifest format and the legacy single-slot format
   * (a raw JSON array). Throws if the data is malformed or a chunk is missing,
   * which the caller treats as corruption.
   */
  private async deserializeStoredCredentials(stored: string): Promise<NFCCredential[]> {
    const parsed: unknown = JSON.parse(stored);

    // Chunked format: a manifest pointing at N chunk slots.
    if (
      parsed !== null &&
      typeof parsed === 'object' &&
      !Array.isArray(parsed) &&
      typeof (parsed as CredentialManifest).chunks === 'number'
    ) {
      const { chunks } = parsed as CredentialManifest;
      const parts: string[] = [];
      for (let i = 0; i < chunks; i++) {
        const part = await SecureStore.getItemAsync(`${CREDENTIALS_CHUNK_PREFIX}${i}`);
        if (part === null) {
          throw new Error(`missing credential chunk ${i} of ${chunks}`);
        }
        parts.push(part);
      }
      return JSON.parse(parts.join('')) as NFCCredential[];
    }

    // Legacy single-slot format: the whole array stringified under the key.
    if (Array.isArray(parsed)) {
      return parsed as NFCCredential[];
    }

    throw new Error('unrecognized stored credential format');
  }

  /** Delete the manifest and every chunk slot it could have written. */
  private async clearStoredCredentials(): Promise<void> {
    await SecureStore.deleteItemAsync(CREDENTIALS_KEY);
    // Deleting a non-existent SecureStore key is a no-op, so a bounded sweep is
    // safe even when we don't know the exact prior chunk count.
    for (let i = 0; i < MAX_CREDENTIAL_CHUNKS; i++) {
      await SecureStore.deleteItemAsync(`${CREDENTIALS_CHUNK_PREFIX}${i}`);
    }
  }

  private async storeCredentials(): Promise<void> {
    const serialized = JSON.stringify(this.credentials);
    const parts = chunkByUtf8Bytes(serialized, MAX_CHUNK_BYTES);

    // How many chunk slots did the previous write leave behind? Needed so a
    // shrinking credential set doesn't strand stale chunks that a later read
    // might reassemble into a corrupt blob.
    const previousChunks = await this.readManifestChunkCount();

    // Write the chunks first, then the manifest that points at them: a crash
    // between the two leaves the (still-consistent) previous manifest in place.
    for (let i = 0; i < parts.length; i++) {
      await SecureStore.setItemAsync(`${CREDENTIALS_CHUNK_PREFIX}${i}`, parts[i]);
    }
    const manifest: CredentialManifest = { v: 1, chunks: parts.length };
    await SecureStore.setItemAsync(CREDENTIALS_KEY, JSON.stringify(manifest));

    // Remove chunk slots left over from a previously larger credential set.
    for (let i = parts.length; i < previousChunks; i++) {
      await SecureStore.deleteItemAsync(`${CREDENTIALS_CHUNK_PREFIX}${i}`);
    }
  }

  /** Chunk count recorded by the last chunked write, or 0 if none/legacy. */
  private async readManifestChunkCount(): Promise<number> {
    const raw = await SecureStore.getItemAsync(CREDENTIALS_KEY);
    if (!raw) return 0;
    try {
      const parsed: unknown = JSON.parse(raw);
      if (
        parsed !== null &&
        typeof parsed === 'object' &&
        !Array.isArray(parsed) &&
        typeof (parsed as CredentialManifest).chunks === 'number'
      ) {
        return (parsed as CredentialManifest).chunks;
      }
    } catch {
      // Not a manifest (legacy raw array or corrupt) — nothing to clean up.
    }
    return 0;
  }

  private async apiRequest<T>(
    endpoint: string,
    method: 'GET' | 'POST' = 'GET',
    body?: unknown
  ): Promise<T> {
    const headers: Record<string, string> = {
      'Content-Type': 'application/json',
    };

    if (this.authToken) {
      headers.Authorization = `Bearer ${this.authToken}`;
    }

    const response = await fetch(`${this.apiBaseUrl}${endpoint}`, {
      method,
      headers,
      body: body ? JSON.stringify(body) : undefined,
    });

    if (!response.ok) {
      throw new Error(`API request failed: ${response.status}`);
    }

    return response.json();
  }
}
