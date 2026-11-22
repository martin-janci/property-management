import { describe, expect, it } from 'vitest';
import { buildAuditQuery, parseAuditPage, toAuditEventView } from './auditApi';

describe('auditApi', () => {
  it('maps a server audit row to the view shape', () => {
    const view = toAuditEventView({
      id: 'r1',
      user_id: 'u1',
      actor_email: 'ops@example.com',
      action: 'resource_accessed',
      resource_type: 'tenant',
      resource_id: 't1',
      details: { capability: 'tenant_purge', outcome: 'denied' },
      ip_address: '10.0.0.1',
      created_at: '2026-11-21T10:00:00Z',
    });
    expect(view).toMatchObject({
      occurred_at: '2026-11-21T10:00:00Z',
      actor_email: 'ops@example.com',
      actor_user_id: 'u1',
      capability: 'tenant_purge',
      outcome: 'denied',
      target_kind: 'tenant',
      target_slug: 't1',
      ip: '10.0.0.1',
      severity: 'high',
    });
  });

  it('falls back to the raw action when details carry no capability', () => {
    const view = toAuditEventView({ id: 'r2', action: 'login', created_at: 'x' });
    expect(view.capability).toBe('login');
    expect(view.severity).toBe('normal');
  });

  it('builds the server query string', () => {
    const qs = new URLSearchParams(
      buildAuditQuery(
        {
          actor: 'ops',
          capabilities: ['a', 'b'],
          severity: 'normal',
          outcome: 'all',
          dateRange: '7d',
        },
        'c1'
      )
    );
    expect(qs.get('actor')).toBe('ops');
    expect(qs.get('capability')).toBe('a,b');
    expect(qs.has('severity')).toBe(false);
    expect(qs.has('outcome')).toBe(false);
    expect(qs.get('range')).toBe('7d');
    expect(qs.get('cursor')).toBe('c1');
  });

  it('accepts a bare array defensively', () => {
    const page = parseAuditPage([{ id: 'r3', action: 'login', created_at: 'x' }]);
    expect(page.items).toHaveLength(1);
    expect(page.next_cursor).toBeUndefined();
  });
});
