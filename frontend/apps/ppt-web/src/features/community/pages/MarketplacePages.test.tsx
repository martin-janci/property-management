/// <reference types="vitest/globals" />
/**
 * Marketplace list / create / detail + inquiry — wired to the real Community
 * client over a mocked `fetch`.
 */

import { screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import {
  BUILDING_ID,
  ME,
  mockApi,
  rawItem,
  renderPage,
  unitsRoute,
} from '../test/communityTestUtils';
import { CreateItemPage } from './CreateItemPage';
import { ItemDetailPage } from './ItemDetailPage';
import { MarketplacePage } from './MarketplacePage';

const h = vi.hoisted(() => ({
  showToast: vi.fn(),
  user: { id: 'user-me', role: 'owner' } as { id: string; role?: string },
}));

vi.mock('../../../contexts', () => ({ useAuth: () => ({ user: h.user }) }));
vi.mock('../../../components', async (importOriginal) => ({
  ...(await importOriginal<typeof import('../../../components')>()),
  useToast: () => ({ showToast: h.showToast }),
}));

const listPath = `/api/v1/community/buildings/${BUILDING_ID}/marketplace`;

beforeEach(() => {
  h.showToast.mockReset();
  h.user = { id: ME, role: 'owner' };
});
afterEach(() => vi.unstubAllGlobals());

describe('MarketplacePage', () => {
  it('lists items, filters by category server-side and opens a listing', async () => {
    const user = userEvent.setup();
    const api = mockApi([
      unitsRoute,
      {
        method: 'GET',
        path: listPath,
        reply: (call) => ({
          body: call.search.includes('category=books')
            ? [
                rawItem({
                  id: 'i-2',
                  title: 'Cookbook',
                  category: 'books',
                  price: null,
                  is_free: true,
                }),
              ]
            : [rawItem()],
        }),
      },
    ]);
    renderPage(<MarketplacePage />, '/community/marketplace', '/community/marketplace');

    expect(await screen.findByText('Oak bookshelf')).toBeInTheDocument();
    expect(screen.getByText('€45.00')).toBeInTheDocument();
    expect(api.find('GET', listPath)[0].search).toBe('?limit=24&offset=0');

    await user.selectOptions(screen.getByLabelText('Category'), 'books');
    expect(await screen.findByText('Cookbook')).toBeInTheDocument();
    expect(screen.getByText('FREE')).toBeInTheDocument();
    expect(api.find('GET', listPath).at(-1)?.search).toBe('?category=books&limit=24&offset=0');

    await user.click(screen.getByRole('button', { name: 'View Details' }));
    expect(screen.getByTestId('location')).toHaveTextContent(
      `/community/marketplace/i-2?building=${BUILDING_ID}`
    );
  });

  it('shows an error state when the list fails', async () => {
    mockApi([unitsRoute, { method: 'GET', path: listPath, reply: { status: 500, body: {} } }]);
    renderPage(<MarketplacePage />, '/community/marketplace', '/community/marketplace');
    expect(await screen.findByRole('alert')).toHaveTextContent('Could not load the data');
  });
});

describe('CreateItemPage', () => {
  it('publishes a listing and opens it', async () => {
    const user = userEvent.setup();
    const api = mockApi([
      unitsRoute,
      { method: 'POST', path: listPath, reply: { status: 201, body: rawItem({ id: 'i-new' }) } },
    ]);
    renderPage(<CreateItemPage />, '/community/marketplace/new', '/community/marketplace/new');

    await user.type(await screen.findByLabelText(/Title/), 'Oak bookshelf');
    await user.selectOptions(screen.getByLabelText('Condition'), 'like_new');
    await user.type(screen.getByLabelText('Price'), '45,5');
    await user.click(screen.getByLabelText('Price negotiable'));
    await user.type(screen.getByLabelText(/Pickup details/), 'Ring flat 3');
    await user.click(screen.getByRole('button', { name: 'Create Listing' }));

    await waitFor(() => expect(api.find('POST', listPath)).toHaveLength(1));
    expect(api.find('POST', listPath)[0].body).toEqual({
      title: 'Oak bookshelf',
      category: 'furniture',
      condition: 'like_new',
      price: '45.50',
      is_negotiable: true,
      pickup_details: 'Ring flat 3',
    });
    await waitFor(() =>
      expect(screen.getByTestId('location')).toHaveTextContent(
        `/community/marketplace/i-new?building=${BUILDING_ID}`
      )
    );
  });
});

describe('ItemDetailPage', () => {
  const route = '/community/marketplace/:itemId';

  it('shows the listing and sends an inquiry to the seller', async () => {
    const user = userEvent.setup();
    const api = mockApi([
      { method: 'GET', path: '/api/v1/community/marketplace/i-1', reply: { body: rawItem() } },
      {
        method: 'POST',
        path: '/api/v1/community/marketplace/i-1/inquiries',
        reply: {
          status: 201,
          body: {
            id: 'q-1',
            item_id: 'i-1',
            buyer_id: ME,
            message: 'Still available?',
            offer_price: '40.00',
            status: 'pending',
            created_at: 't',
          },
        },
      },
    ]);
    renderPage(<ItemDetailPage />, '/community/marketplace/i-1', route);

    expect(await screen.findByRole('heading', { name: 'Oak bookshelf' })).toBeInTheDocument();
    expect(screen.getByText('Price negotiable')).toBeInTheDocument();

    await user.type(screen.getByLabelText(/Message/), 'Still available?');
    await user.type(screen.getByLabelText(/Your offer/), '40');
    await user.click(screen.getByRole('button', { name: 'Send message' }));

    await waitFor(() =>
      expect(api.find('POST', '/api/v1/community/marketplace/i-1/inquiries')[0]?.body).toEqual({
        message: 'Still available?',
        offer_price: '40.00',
      })
    );
    expect(await screen.findByRole('status')).toHaveTextContent(
      'Your message was sent to the seller.'
    );
  });

  it('requires a message', async () => {
    const user = userEvent.setup();
    const api = mockApi([
      { method: 'GET', path: '/api/v1/community/marketplace/i-1', reply: { body: rawItem() } },
    ]);
    renderPage(<ItemDetailPage />, '/community/marketplace/i-1', route);
    await user.click(await screen.findByRole('button', { name: 'Send message' }));
    expect(screen.getByText('Please write a message.')).toBeInTheDocument();
    expect(api.find('POST', '/api/v1/community/marketplace/i-1/inquiries')).toHaveLength(0);
  });

  it('shows the seller their own listing without a contact form', async () => {
    mockApi([
      {
        method: 'GET',
        path: '/api/v1/community/marketplace/i-1',
        reply: { body: rawItem({ seller_id: ME }) },
      },
    ]);
    renderPage(<ItemDetailPage />, '/community/marketplace/i-1', route);
    expect(await screen.findByText(/This is your listing/)).toBeInTheDocument();
    expect(screen.queryByRole('button', { name: 'Send message' })).not.toBeInTheDocument();
  });

  it('renders not-found for a missing listing', async () => {
    mockApi([
      {
        method: 'GET',
        path: '/api/v1/community/marketplace/i-1',
        reply: { status: 404, body: { code: 'NOT_FOUND', message: 'Item not found' } },
      },
    ]);
    renderPage(<ItemDetailPage />, '/community/marketplace/i-1', route);
    expect(await screen.findByText('Listing not found')).toBeInTheDocument();
  });
});
