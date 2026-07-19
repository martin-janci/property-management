import type { ListingDetail } from '@ppt/reality-api-client';
import { render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import type { ResolvedScreen } from '../../lib/layout';
import { LayoutSections } from './LayoutSections';
import type { ListingRegistry } from './sections/registry';

// next-intl is already mocked globally in src/test/setup.tsx (key-echo)

const dummyListing = { id: 'test-listing-1' } as ListingDetail;

const registry: ListingRegistry = {
  'alpha.v1': { component: () => <div>ALPHA</div>, required: true, supportedModes: [] },
  'beta.v1': {
    component: ({ mode }) => <div>BETA:{mode ?? 'none'}</div>,
    required: false,
    supportedModes: ['list', 'grid'],
  },
  'boom.v1': {
    component: () => {
      throw new Error('section crash');
    },
    required: false,
    supportedModes: [],
  },
};

function layoutOf(sections: ResolvedScreen['sections']): ResolvedScreen {
  return { screen: 'test/screen', version: 1, sections };
}

describe('LayoutSections', () => {
  it('renders sections in resolved order with mode passed through', () => {
    render(
      <LayoutSections
        registry={registry}
        layout={layoutOf([
          { type: 'beta.v1', mode: 'grid', presentation: 'visible' },
          { type: 'alpha.v1', presentation: 'visible' },
        ])}
        listing={dummyListing}
      />
    );
    const texts = screen.getAllByText(/ALPHA|BETA/).map((n) => n.textContent);
    expect(texts).toEqual(['BETA:grid', 'ALPHA']);
  });

  it('renders a placeholder for presentation=placeholder and no component output', () => {
    render(
      <LayoutSections
        registry={registry}
        layout={layoutOf([{ type: 'alpha.v1', presentation: 'placeholder' }])}
        listing={dummyListing}
      />
    );
    expect(screen.queryByText('ALPHA')).toBeNull();
    expect(screen.getByRole('status')).toHaveTextContent('placeholderTitle');
  });

  it('skips unknown section types entirely and warns once per type', () => {
    const warn = vi.spyOn(console, 'warn').mockImplementation(() => {});
    render(
      <LayoutSections
        registry={registry}
        layout={layoutOf([
          { type: 'specter.v77', presentation: 'visible' },
          { type: 'specter.v77', presentation: 'visible' },
          { type: 'alpha.v1', presentation: 'visible' },
        ])}
        listing={dummyListing}
      />
    );
    expect(screen.getByText('ALPHA')).toBeInTheDocument();
    expect(warn.mock.calls.filter((c) => String(c[0]).includes('specter.v77'))).toHaveLength(1);
    warn.mockRestore();
  });

  it('isolates a crashing section: placeholder for it, siblings render', () => {
    const err = vi.spyOn(console, 'error').mockImplementation(() => {});
    render(
      <LayoutSections
        registry={registry}
        layout={layoutOf([
          { type: 'boom.v1', presentation: 'visible' },
          { type: 'alpha.v1', presentation: 'visible' },
        ])}
        listing={dummyListing}
      />
    );
    expect(screen.getByText('ALPHA')).toBeInTheDocument();
    expect(screen.getByRole('status')).toBeInTheDocument();
    err.mockRestore();
  });
});
