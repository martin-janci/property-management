/**
 * Authentication utilities for Property Management apps.
 * Provides token storage, JWT helpers, and common auth utilities.
 */

// ============================================
// Types
// ============================================

/**
 * Decoded JWT payload structure.
 */
export interface JwtPayload {
  /** Subject (user ID) */
  sub: string;
  /** Email address */
  email?: string;
  /** User's display name */
  name?: string;
  /** Tenant ID */
  tenant_id?: string;
  /** User role */
  role?: string;
  /** Issued at timestamp (seconds since epoch) */
  iat: number;
  /** Expiration timestamp (seconds since epoch) */
  exp: number;
}

/**
 * Token pair from authentication.
 */
export interface TokenPair {
  accessToken: string;
  refreshToken: string;
}

/**
 * Basic user info extracted from JWT.
 */
export interface BasicUserInfo {
  id: string;
  email?: string;
  name?: string;
  tenantId?: string;
  role?: string;
}

// ============================================
// JWT Utilities
// ============================================

/**
 * Decode a JWT token without verification.
 * NOTE: This does NOT verify the signature - use only for reading claims.
 *
 * @param token - The JWT token string
 * @returns Decoded payload or null if invalid
 */
export function decodeJwt(token: string): JwtPayload | null {
  try {
    const parts = token.split('.');
    if (parts.length !== 3) {
      return null;
    }

    const payload = parts[1];
    // Handle URL-safe base64
    const base64 = payload.replace(/-/g, '+').replace(/_/g, '/');
    const jsonPayload = decodeURIComponent(
      atob(base64)
        .split('')
        .map((c) => `%${c.charCodeAt(0).toString(16).padStart(2, '0')}`)
        .join('')
    );

    return JSON.parse(jsonPayload) as JwtPayload;
  } catch {
    return null;
  }
}

/**
 * Check if a JWT token is expired.
 *
 * @param token - The JWT token string
 * @param bufferSeconds - Seconds before actual expiry to consider expired (default: 30)
 * @returns true if expired or invalid, false if still valid
 */
export function isTokenExpired(token: string, bufferSeconds = 30): boolean {
  const payload = decodeJwt(token);
  if (!payload) {
    return true;
  }

  const now = Math.floor(Date.now() / 1000);
  return payload.exp <= now + bufferSeconds;
}

/**
 * Get the expiration time of a JWT token.
 *
 * @param token - The JWT token string
 * @returns Expiration Date or null if invalid
 */
export function getTokenExpiration(token: string): Date | null {
  const payload = decodeJwt(token);
  if (!payload) {
    return null;
  }

  return new Date(payload.exp * 1000);
}

/**
 * Get remaining time until token expires in seconds.
 *
 * @param token - The JWT token string
 * @returns Seconds until expiry, 0 if expired, -1 if invalid
 */
export function getTokenRemainingTime(token: string): number {
  const payload = decodeJwt(token);
  if (!payload) {
    return -1;
  }

  const now = Math.floor(Date.now() / 1000);
  return Math.max(0, payload.exp - now);
}

/**
 * Extract basic user info from a JWT token.
 *
 * @param token - The JWT token string
 * @returns User info or null if invalid
 */
export function getUserFromToken(token: string): BasicUserInfo | null {
  const payload = decodeJwt(token);
  if (!payload) {
    return null;
  }

  return {
    id: payload.sub,
    email: payload.email,
    name: payload.name,
    tenantId: payload.tenant_id,
    role: payload.role,
  };
}

// ============================================
// Token Storage (Browser)
// ============================================

/**
 * Create a token storage instance with the given key prefix.
 * Uses localStorage with graceful fallback for SSR.
 */
export function createTokenStorage(prefix: string) {
  const ACCESS_TOKEN_KEY = `${prefix}_access_token`;
  const REFRESH_TOKEN_KEY = `${prefix}_refresh_token`;

  const isAvailable = (): boolean => {
    try {
      if (typeof window === 'undefined' || !window.localStorage) {
        return false;
      }
      const testKey = '__storage_test__';
      localStorage.setItem(testKey, testKey);
      localStorage.removeItem(testKey);
      return true;
    } catch {
      return false;
    }
  };

  return {
    /**
     * Get the stored access token.
     */
    getAccessToken(): string | null {
      if (!isAvailable()) return null;
      try {
        return localStorage.getItem(ACCESS_TOKEN_KEY);
      } catch {
        return null;
      }
    },

    /**
     * Store the access token.
     */
    setAccessToken(token: string): void {
      if (!isAvailable()) return;
      try {
        localStorage.setItem(ACCESS_TOKEN_KEY, token);
      } catch {
        // Storage full or blocked
      }
    },

    /**
     * Get the stored refresh token.
     */
    getRefreshToken(): string | null {
      if (!isAvailable()) return null;
      try {
        return localStorage.getItem(REFRESH_TOKEN_KEY);
      } catch {
        return null;
      }
    },

    /**
     * Store the refresh token.
     */
    setRefreshToken(token: string): void {
      if (!isAvailable()) return;
      try {
        localStorage.setItem(REFRESH_TOKEN_KEY, token);
      } catch {
        // Storage full or blocked
      }
    },

    /**
     * Store both tokens.
     */
    setTokens(tokens: TokenPair): void {
      this.setAccessToken(tokens.accessToken);
      this.setRefreshToken(tokens.refreshToken);
    },

    /**
     * Get both tokens.
     */
    getTokens(): TokenPair | null {
      const accessToken = this.getAccessToken();
      const refreshToken = this.getRefreshToken();
      if (!accessToken || !refreshToken) {
        return null;
      }
      return { accessToken, refreshToken };
    },

    /**
     * Clear all tokens.
     */
    clearTokens(): void {
      if (!isAvailable()) return;
      try {
        localStorage.removeItem(ACCESS_TOKEN_KEY);
        localStorage.removeItem(REFRESH_TOKEN_KEY);
      } catch {
        // Storage blocked
      }
    },

    /**
     * Check if tokens are stored and access token is valid.
     */
    hasValidTokens(): boolean {
      const accessToken = this.getAccessToken();
      if (!accessToken) return false;
      return !isTokenExpired(accessToken);
    },

    /**
     * Check if refresh token is available (even if access token expired).
     */
    canRefresh(): boolean {
      const refreshToken = this.getRefreshToken();
      if (!refreshToken) return false;
      return !isTokenExpired(refreshToken);
    },
  };
}

// ============================================
// Auth Header Helpers
// ============================================

/**
 * Create an Authorization header value from a token.
 */
export function createAuthHeader(token: string): string {
  return `Bearer ${token}`;
}

/**
 * Extract token from Authorization header.
 */
export function extractTokenFromHeader(header: string): string | null {
  if (!header.startsWith('Bearer ')) {
    return null;
  }
  return header.slice(7);
}

// ============================================
// Role Helpers
// ============================================

/**
 * Common roles in the property management system.
 */
export const ROLES = {
  ADMIN: 'admin',
  MANAGER: 'manager',
  OWNER: 'owner',
  RESIDENT: 'resident',
  VIEWER: 'viewer',
} as const;

export type Role = (typeof ROLES)[keyof typeof ROLES];

/**
 * Check if a role has permission based on hierarchy.
 * Admin > Manager > Owner > Resident > Viewer
 */
export function hasRolePermission(userRole: string, requiredRole: Role): boolean {
  const hierarchy: Role[] = ['viewer', 'resident', 'owner', 'manager', 'admin'];
  const userLevel = hierarchy.indexOf(userRole as Role);
  const requiredLevel = hierarchy.indexOf(requiredRole);

  if (userLevel === -1 || requiredLevel === -1) {
    return false;
  }

  return userLevel >= requiredLevel;
}

/**
 * Check if user has any of the required roles.
 */
export function hasAnyRole(userRole: string, roles: Role[]): boolean {
  return roles.some((role) => userRole === role);
}

// ============================================
// Session Storage Helpers
// ============================================

/**
 * Store return URL for post-login redirect.
 */
export function setReturnUrl(url: string): void {
  if (typeof sessionStorage === 'undefined') return;
  try {
    sessionStorage.setItem('auth_return_url', url);
  } catch {
    // Storage blocked or full
  }
}

/**
 * Get and clear the stored return URL.
 */
export function getAndClearReturnUrl(): string | null {
  if (typeof sessionStorage === 'undefined') return null;
  try {
    const url = sessionStorage.getItem('auth_return_url');
    sessionStorage.removeItem('auth_return_url');
    return url;
  } catch {
    return null;
  }
}
