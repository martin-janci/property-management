/**
 * Application shell: public header for anonymous visitors, and a grouped,
 * role-aware sidebar + top bar for signed-in users.
 *
 * Replaces the former single flat nav bar that listed ~20 links (half of them
 * English-only, several operator-only) to everyone — including anonymous
 * visitors — while omitting core modules such as Faults, Voting, Financial and
 * Announcements altogether.
 */
import {
  type KeyboardEvent as ReactKeyboardEvent,
  type ReactNode,
  useEffect,
  useId,
  useRef,
  useState,
} from 'react';
import { useTranslation } from 'react-i18next';
import { Link, NavLink, useLocation, useNavigate } from 'react-router-dom';
import { useAuth } from '../../contexts';
import { isManagerRole } from '../../routes/shared';
import { ConnectionStatus } from '../ConnectionStatus';
import { LanguageSwitcher } from '../LanguageSwitcher';
import { NotificationBell } from '../NotificationBell';
import { ACCOUNT_ITEMS, isVisibleFor, sectionsFor } from './navConfig';
import './AppShell.css';

function Brand() {
  const { t } = useTranslation();
  return (
    <Link to="/" className="shell-brand">
      <span className="shell-brand__mark" aria-hidden="true">
        PPT
      </span>
      <span className="shell-brand__name">
        {t('shell.brand', { defaultValue: 'Property Manager' })}
      </span>
    </Link>
  );
}

function initialsOf(first?: string, last?: string, email?: string): string {
  const fromName = `${first?.[0] ?? ''}${last?.[0] ?? ''}`.trim();
  if (fromName) return fromName.toUpperCase();
  return (email?.[0] ?? '?').toUpperCase();
}

function UserMenu({ isManager }: { isManager: boolean }) {
  const { t } = useTranslation();
  const { user, logout } = useAuth();
  const navigate = useNavigate();
  const { pathname } = useLocation();
  const [open, setOpen] = useState(false);
  const menuId = useId();
  const rootRef = useRef<HTMLDivElement>(null);
  const triggerRef = useRef<HTMLButtonElement>(null);
  const menuRef = useRef<HTMLDivElement>(null);
  // When the menu is opened via ArrowUp, focus lands on the last item (WAI-ARIA
  // menu-button pattern); any other open path focuses the first item.
  const openToLastRef = useRef(false);

  const menuItems = () =>
    menuRef.current
      ? Array.from(menuRef.current.querySelectorAll<HTMLElement>('[role="menuitem"]'))
      : [];

  // Close on navigation.
  // biome-ignore lint/correctness/useExhaustiveDependencies: pathname is the trigger
  useEffect(() => setOpen(false), [pathname]);

  // On open, move focus into the menu (first item, or last when opened via
  // ArrowUp) per the WAI-ARIA menu pattern.
  // biome-ignore lint/correctness/useExhaustiveDependencies: menuItems reads a stable ref
  useEffect(() => {
    if (!open) return;
    const items = menuItems();
    const target = openToLastRef.current ? items[items.length - 1] : items[0];
    openToLastRef.current = false;
    target?.focus();
  }, [open]);

  // Close on outside click / Escape. Escape also returns focus to the trigger
  // (WAI-ARIA: Escape returns focus to the menu button; an outside click does
  // not).
  useEffect(() => {
    if (!open) return;
    const onDown = (e: MouseEvent) => {
      if (rootRef.current && !rootRef.current.contains(e.target as Node)) setOpen(false);
    };
    const onKey = (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        setOpen(false);
        triggerRef.current?.focus();
      }
    };
    document.addEventListener('mousedown', onDown);
    document.addEventListener('keydown', onKey);
    return () => {
      document.removeEventListener('mousedown', onDown);
      document.removeEventListener('keydown', onKey);
    };
  }, [open]);

  // Roving focus within the open menu.
  const onMenuKeyDown = (e: ReactKeyboardEvent<HTMLDivElement>) => {
    const items = menuItems();
    if (items.length === 0) return;
    const active = document.activeElement as HTMLElement | null;
    const index = active ? items.indexOf(active) : -1;
    switch (e.key) {
      case 'ArrowDown':
        e.preventDefault();
        items[(index + 1) % items.length]?.focus();
        break;
      case 'ArrowUp':
        e.preventDefault();
        items[(index - 1 + items.length) % items.length]?.focus();
        break;
      case 'Home':
        e.preventDefault();
        items[0]?.focus();
        break;
      case 'End':
        e.preventDefault();
        items[items.length - 1]?.focus();
        break;
      case 'Tab':
        // Tabbing out of the menu closes it but lets focus proceed naturally.
        setOpen(false);
        break;
      default:
        break;
    }
  };

  // Open the menu from the trigger with the arrow keys, landing focus on the
  // first (ArrowDown) or last (ArrowUp) item.
  const onTriggerKeyDown = (e: ReactKeyboardEvent<HTMLButtonElement>) => {
    if (e.key === 'ArrowDown' || e.key === 'ArrowUp') {
      e.preventDefault();
      openToLastRef.current = e.key === 'ArrowUp';
      setOpen(true);
    }
  };

  const displayName =
    [user?.firstName, user?.lastName].filter(Boolean).join(' ') || user?.email || '';
  const roleLabel = user?.role
    ? t(`shell.roles.${user.role}`, { defaultValue: user.role.replace(/_/g, ' ') })
    : '';

  const handleLogout = async () => {
    await logout();
    navigate('/login', { replace: true });
  };

  return (
    <div className="shell-user" ref={rootRef}>
      <button
        ref={triggerRef}
        type="button"
        className="shell-user__trigger"
        aria-haspopup="menu"
        aria-expanded={open}
        aria-controls={menuId}
        onClick={() => setOpen((v) => !v)}
        onKeyDown={onTriggerKeyDown}
      >
        <span className="shell-user__avatar" aria-hidden="true">
          {initialsOf(user?.firstName, user?.lastName, user?.email)}
        </span>
        <span className="shell-user__meta">
          <span className="shell-user__name">{displayName}</span>
          {roleLabel && <span className="shell-user__role">{roleLabel}</span>}
        </span>
      </button>
      {open && (
        <div
          className="shell-user__menu"
          id={menuId}
          role="menu"
          ref={menuRef}
          onKeyDown={onMenuKeyDown}
        >
          {user?.organizationName && (
            <div className="shell-user__org" role="presentation">
              {user.organizationName}
            </div>
          )}
          {ACCOUNT_ITEMS.filter((item) => isVisibleFor(item, isManager)).map((item) => (
            <Link key={item.to} to={item.to} role="menuitem" className="shell-user__item">
              {t(item.labelKey, { defaultValue: item.defaultLabel })}
            </Link>
          ))}
          <hr className="shell-user__divider" />
          <button
            type="button"
            role="menuitem"
            className="shell-user__item shell-user__item--danger"
            onClick={handleLogout}
          >
            {t('auth.signOut', { defaultValue: 'Sign out' })}
          </button>
        </div>
      )}
    </div>
  );
}

function Sidebar({
  isManager,
  open,
  onClose,
}: {
  isManager: boolean;
  open: boolean;
  onClose: () => void;
}) {
  const { t } = useTranslation();
  const sections = sectionsFor(isManager);

  return (
    <>
      {open && (
        <button
          type="button"
          className="shell-backdrop"
          aria-label={t('aria.closeMenu')}
          onClick={onClose}
        />
      )}
      <nav
        id="app-sidebar"
        className={`shell-sidebar${open ? ' shell-sidebar--open' : ''}`}
        aria-label={t('aria.mainNavigation')}
      >
        {sections.map((section) => (
          <div key={section.id} className="shell-nav-section">
            <div className="shell-nav-section__title">
              {t(section.labelKey, { defaultValue: section.defaultLabel })}
            </div>
            <ul>
              {section.items.map((item) => (
                <li key={item.to}>
                  <NavLink
                    to={item.to}
                    end={item.end}
                    className={({ isActive }) =>
                      `shell-nav-link${isActive ? ' shell-nav-link--active' : ''}`
                    }
                  >
                    {t(item.labelKey, { defaultValue: item.defaultLabel })}
                  </NavLink>
                </li>
              ))}
            </ul>
          </div>
        ))}
      </nav>
    </>
  );
}

function PublicHeader() {
  const { t } = useTranslation();
  return (
    <header className="shell-topbar shell-topbar--public">
      <Brand />
      <nav
        className="shell-topbar__actions"
        data-testid="primary-nav"
        aria-label={t('aria.publicNavigation')}
      >
        <LanguageSwitcher />
        <Link to="/login" className="shell-btn shell-btn--ghost">
          {t('auth.signIn')}
        </Link>
        <Link to="/register" className="shell-btn shell-btn--primary">
          {t('shell.createAccount', { defaultValue: 'Create account' })}
        </Link>
      </nav>
    </header>
  );
}

/**
 * Wraps the routed content. Anonymous visitors get a slim public header;
 * authenticated users get the sidebar layout.
 */
export function AppShell({ children }: { children: ReactNode }) {
  const { t } = useTranslation();
  const { isAuthenticated, user } = useAuth();
  const isManager = isManagerRole(user?.role);
  const [sidebarOpen, setSidebarOpen] = useState(false);
  const { pathname } = useLocation();

  // biome-ignore lint/correctness/useExhaustiveDependencies: pathname is the trigger
  useEffect(() => setSidebarOpen(false), [pathname]);

  if (!isAuthenticated) {
    return (
      <div className="app shell shell--public">
        <PublicHeader />
        {children}
      </div>
    );
  }

  return (
    <div className="app shell shell--authed">
      <header className="shell-topbar">
        <button
          type="button"
          className="shell-menu-toggle"
          aria-controls="app-sidebar"
          aria-expanded={sidebarOpen}
          aria-label={t('aria.openMenu')}
          onClick={() => setSidebarOpen((v) => !v)}
        >
          <span aria-hidden="true">☰</span>
        </button>
        <Brand />
        <div className="shell-topbar__actions">
          <ConnectionStatus />
          <NotificationBell />
          <LanguageSwitcher />
          <UserMenu isManager={isManager} />
        </div>
      </header>
      <div className="shell-body">
        <Sidebar isManager={isManager} open={sidebarOpen} onClose={() => setSidebarOpen(false)} />
        <div className="shell-content">{children}</div>
      </div>
    </div>
  );
}
