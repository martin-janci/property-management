/**
 * ContactForm — listing-inquiry submit wiring (#code-review reality-web agent
 * contact form does not persist).
 *
 * Before the fix, `useCreateInquiry` POSTed to `/api/v1/inquiries` — a path the
 * reality-server only serves for `GET` (list my inquiries). There is no `POST`
 * route there, so every contact-form submission was silently dropped (405/404)
 * while the form still flipped to its success screen. The real public endpoint
 * is listing-scoped: `POST /api/v1/inquiries/contact/{listing_id}` with a
 * `ContactMessageRequest` body (`name` / `email` / `phone` / `message`).
 *
 * These tests render the REAL ContactForm with the REAL `useCreateInquiry`
 * (only `fetch` is stubbed) and pin:
 *   1. a valid submit POSTs the listing-scoped contact URL with the snake_case
 *      body the server expects, and only then shows the success screen;
 *   2. a failed POST surfaces the error state instead of a fake success.
 *
 * On `dev` test (1) fails: the POST goes to the bare `/api/v1/inquiries`
 * collection path, so the `toMatch(/\/contact\/<listingId>$/)` assertion fails
 * — this is the IG3 regression guard.
 *
 * next-intl is mocked globally by src/test/setup.tsx (`t(key) => key`), so the
 * assertions below pin message KEYS rather than any one locale's copy.
 */
/// <reference types="vitest/globals" />

import type { ListingAgent } from '@ppt/reality-api-client';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { ContactForm } from './ContactForm';

const LISTING_ID = '11111111-2222-3333-4444-555555555555';

const AGENT: ListingAgent = {
  id: 'agent-1',
  name: 'Jane Agent',
  email: 'jane@example.com',
  agencyName: 'Acme Reality',
};

function renderForm() {
  const client = new QueryClient({
    defaultOptions: { queries: { retry: false }, mutations: { retry: false } },
  });
  return render(
    <QueryClientProvider client={client}>
      <ContactForm listingId={LISTING_ID} agent={AGENT} />
    </QueryClientProvider>
  );
}

function fillAndSubmit(container: HTMLElement) {
  const name = container.querySelector('#name') as HTMLInputElement;
  const email = container.querySelector('#email') as HTMLInputElement;
  const message = container.querySelector('#message') as HTMLTextAreaElement;
  fireEvent.change(name, { target: { value: 'John Buyer' } });
  fireEvent.change(email, { target: { value: 'john@example.com' } });
  fireEvent.change(message, { target: { value: 'Is this still available?' } });
  const form = container.querySelector('form');
  if (!form) throw new Error('contact form not found');
  fireEvent.submit(form);
}

describe('ContactForm — listing-inquiry submit wiring', () => {
  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('POSTs the listing-scoped contact endpoint with the server body, then shows success', async () => {
    const fetchMock = vi.fn((_url: string, _init?: RequestInit) =>
      Promise.resolve({
        ok: true,
        status: 201,
        json: async () => ({
          message: 'Your message has been sent.',
          inquiry_id: 'a0b1c2d3-e4f5-6789-abcd-ef0123456789',
        }),
      })
    );
    vi.stubGlobal('fetch', fetchMock);

    const { container } = renderForm();
    fillAndSubmit(container);

    await waitFor(() => {
      const post = fetchMock.mock.calls.find(([, init]) => init?.method === 'POST');
      expect(post).toBeDefined();
      // Must hit the listing-scoped contact route — NOT the bare collection
      // path, which only serves GET and never persists the inquiry.
      expect(String(post?.[0])).toMatch(new RegExp(`/api/v1/inquiries/contact/${LISTING_ID}$`));
      const body = JSON.parse(String(post?.[1]?.body));
      expect(body).toEqual({
        name: 'John Buyer',
        email: 'john@example.com',
        message: 'Is this still available?',
      });
    });

    expect(await screen.findByText('messageSent')).toBeInTheDocument();
  });

  it('surfaces the error state (not a fake success) when the POST fails', async () => {
    const fetchMock = vi.fn((_url: string, _init?: RequestInit) =>
      Promise.resolve({ ok: false, status: 500, json: async () => ({}) })
    );
    vi.stubGlobal('fetch', fetchMock);

    const { container } = renderForm();
    fillAndSubmit(container);

    expect(await screen.findByText('failedToSend')).toBeInTheDocument();
    expect(screen.queryByText('messageSent')).not.toBeInTheDocument();
  });
});
