'use client';

/**
 * Authentication button component (Epic 10A-SSO).
 * Shows login button when not authenticated, user menu when authenticated.
 */

import { useAuth } from '@/lib/auth-context';
import { type CSSProperties, useState } from 'react';

const styles: Record<string, CSSProperties> = {
  skeleton: {
    height: '40px',
    width: '96px',
    backgroundColor: 'var(--ppt-border-default)',
    borderRadius: '8px',
  },
  signInButton: {
    padding: '8px 16px',
    backgroundColor: 'var(--ppt-color-primary)',
    color: 'var(--ppt-fg-on-accent)',
    borderRadius: '8px',
    border: 'none',
    cursor: 'pointer',
    fontSize: '14px',
    fontWeight: 500,
  },
  userContainer: {
    position: 'relative',
  },
  userButton: {
    display: 'flex',
    alignItems: 'center',
    gap: '8px',
    padding: '8px 16px',
    borderRadius: '8px',
    border: 'none',
    backgroundColor: 'transparent',
    cursor: 'pointer',
  },
  avatar: {
    width: '32px',
    height: '32px',
    borderRadius: '50%',
    backgroundColor: 'var(--ppt-color-primary)',
    color: 'var(--ppt-fg-on-accent)',
    display: 'flex',
    alignItems: 'center',
    justifyContent: 'center',
    fontWeight: 600,
    fontSize: '14px',
  },
  userName: {
    fontSize: '14px',
    fontWeight: 500,
    color: 'var(--ppt-fg-secondary)',
  },
  dropdown: {
    position: 'absolute',
    right: 0,
    top: '100%',
    marginTop: '8px',
    width: '192px',
    backgroundColor: 'var(--ppt-bg-surface)',
    borderRadius: '8px',
    boxShadow: 'var(--ppt-shadow-lg)',
    border: '1px solid var(--ppt-border-default)',
    zIndex: 10,
  },
  dropdownHidden: {
    display: 'none',
  },
  dropdownHeader: {
    padding: '12px',
    borderBottom: '1px solid var(--ppt-border-default)',
  },
  dropdownName: {
    fontSize: '14px',
    fontWeight: 500,
    color: 'var(--ppt-fg-primary)',
  },
  dropdownEmail: {
    fontSize: '12px',
    color: 'var(--ppt-fg-muted)',
  },
  dropdownMenu: {
    padding: '4px',
  },
  menuItem: {
    display: 'block',
    width: '100%',
    padding: '8px 12px',
    fontSize: '14px',
    color: 'var(--ppt-fg-secondary)',
    textDecoration: 'none',
    borderRadius: '4px',
    textAlign: 'left' as const,
    border: 'none',
    backgroundColor: 'transparent',
    cursor: 'pointer',
  },
  signOutButton: {
    display: 'block',
    width: '100%',
    padding: '8px 12px',
    fontSize: '14px',
    color: 'var(--ppt-color-danger-hover)',
    textAlign: 'left' as const,
    border: 'none',
    backgroundColor: 'transparent',
    cursor: 'pointer',
    borderRadius: '4px',
  },
};

export function AuthButton() {
  const { user, isLoading, isAuthenticated, login, logout } = useAuth();
  const [showDropdown, setShowDropdown] = useState(false);

  if (isLoading) {
    return <div style={styles.skeleton} />;
  }

  if (!isAuthenticated) {
    return (
      <button type="button" onClick={() => login()} style={styles.signInButton}>
        Sign In
      </button>
    );
  }

  return (
    <div
      style={styles.userContainer}
      onMouseEnter={() => setShowDropdown(true)}
      onMouseLeave={() => setShowDropdown(false)}
    >
      <button type="button" style={styles.userButton}>
        {user?.avatar_url ? (
          <img
            src={user.avatar_url}
            alt={user.name}
            style={{ ...styles.avatar, backgroundColor: 'transparent' }}
          />
        ) : (
          <div style={styles.avatar}>{user?.name.charAt(0).toUpperCase()}</div>
        )}
        <span style={styles.userName}>{user?.name}</span>
      </button>

      {/* Dropdown menu */}
      <div style={{ ...styles.dropdown, ...(showDropdown ? {} : styles.dropdownHidden) }}>
        <div style={styles.dropdownHeader}>
          <p style={styles.dropdownName}>{user?.name}</p>
          <p style={styles.dropdownEmail}>{user?.email}</p>
        </div>
        <div style={styles.dropdownMenu}>
          <a href="/favorites" style={styles.menuItem}>
            My Favorites
          </a>
          <a href="/profile" style={styles.menuItem}>
            Profile
          </a>
          <button type="button" onClick={logout} style={styles.signOutButton}>
            Sign Out
          </button>
        </div>
      </div>
    </div>
  );
}
