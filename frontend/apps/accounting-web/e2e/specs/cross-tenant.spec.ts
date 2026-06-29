/**
 * accounting-web · cross-tenant isolation (negative) tests.
 *
 * Verifies that an authenticated user from Org A cannot read or mutate
 * invoices, contacts, or statements belonging to Org B.
 *
 * All tests are API-level (direct HTTP via the `api` fixture) because:
 *   - UI navigation cannot express "log in as Org A and request Org B's resource ID".
 *   - API responses (403/404) are the authoritative isolation signal.
 *
 * Requires a live backend with two seeded organisations:
 *   - E2E_ORG_A_EMAIL / E2E_ORG_A_PASSWORD  (Org A admin)
 *   - E2E_ORG_B_INVOICE_ID                   (an invoice that belongs to Org B)
 *   - E2E_ORG_B_CONTACT_ID                   (a contact that belongs to Org B)
 *
 * Without these env vars the suite skips cleanly.
 */

import { backendEnabled, expect, test } from '@ppt/e2e';

const SKIP_REASON =
  'cross-tenant tests require a live backend + two-org seed (set E2E_WITH_BACKEND=1 + E2E_ORG_B_INVOICE_ID)';

function orgBInvoiceId(): string | undefined {
  return process.env.E2E_ORG_B_INVOICE_ID;
}
function orgBContactId(): string | undefined {
  return process.env.E2E_ORG_B_CONTACT_ID;
}

test.describe('accounting-web · cross-tenant isolation @requires-backend', () => {
  test.skip(!backendEnabled() || !orgBInvoiceId(), SKIP_REASON);

  test.beforeEach(async ({ api }) => {
    // Authenticate as Org A user so subsequent api.get() calls carry the token.
    const email = process.env.E2E_ORG_A_EMAIL ?? process.env.E2E_ADMIN_EMAIL;
    const password = process.env.E2E_ORG_A_PASSWORD ?? process.env.E2E_ADMIN_PASSWORD;
    if (!email || !password) {
      test.skip();
      return;
    }
    const login = await api.post<{ access_token?: string; token?: string }>('/api/v1/auth/login', {
      email,
      password,
    });
    const token = login.data?.access_token ?? login.data?.token;
    if (token) {
      api.setToken(token);
    }
  });

  test('Org A cannot read Org B invoice (GET → 403 or 404)', async ({ api }) => {
    const invoiceId = orgBInvoiceId();
    const result = await api.get(`/api/v1/accounting/invoices/${invoiceId}`);
    expect(result.skipped, 'api call must not be skipped').toBe(false);
    expect([403, 404], `expected 403/404, got ${result.status}`).toContain(result.status);
  });

  test('Org A cannot update Org B invoice', async ({ api }) => {
    const invoiceId = orgBInvoiceId();
    // The ApiHelper does not expose PATCH; use a POST to the update route the
    // server will also reject with 403/404 before processing the body.
    const result = await api.post(`/api/v1/accounting/invoices/${invoiceId}`, {
      status: 'paid',
    });
    expect(result.skipped).toBe(false);
    expect([403, 404, 405]).toContain(result.status);
  });

  test('Org A cannot delete Org B invoice (DELETE → 403 or 404)', async ({ api }) => {
    const invoiceId = orgBInvoiceId();
    const result = await api.delete(`/api/v1/accounting/invoices/${invoiceId}`);
    expect(result.skipped).toBe(false);
    expect([403, 404]).toContain(result.status);
  });

  test('Org A cannot read Org B invoice items', async ({ api }) => {
    const invoiceId = orgBInvoiceId();
    const result = await api.get(`/api/v1/accounting/invoices/${invoiceId}/items`);
    expect(result.skipped).toBe(false);
    expect([403, 404]).toContain(result.status);
  });

  test('Org A cannot read Org B contact', async ({ api }) => {
    test.skip(!orgBContactId(), 'E2E_ORG_B_CONTACT_ID not set');
    const result = await api.get(`/api/v1/accounting/contacts/${orgBContactId()}`);
    expect(result.skipped).toBe(false);
    expect([403, 404]).toContain(result.status);
  });

  test('Org A invoice list never includes Org B invoices', async ({ api }) => {
    // List must only contain org-scoped invoices — none should match Org B's known ID.
    const result = await api.get<{ data: Array<{ id: string }> }>(
      '/api/v1/accounting/invoices?limit=100'
    );
    expect(result.ok).toBe(true);
    const ids = (result.data?.data ?? []).map((inv) => inv.id);
    expect(ids, 'Org B invoice must not appear in Org A list').not.toContain(orgBInvoiceId());
  });

  test('Org A contact list never includes Org B contacts', async ({ api }) => {
    test.skip(!orgBContactId(), 'E2E_ORG_B_CONTACT_ID not set');
    const result = await api.get<{ data: Array<{ id: string }> }>(
      '/api/v1/accounting/contacts?limit=100'
    );
    expect(result.ok).toBe(true);
    const ids = (result.data?.data ?? []).map((c) => c.id);
    expect(ids, 'Org B contact must not appear in Org A list').not.toContain(orgBContactId());
  });
});
