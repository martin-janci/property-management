import type { ResolvedScreen } from '@ppt/api-client';
import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';
import { LayoutRenderer } from './LayoutRenderer';
import type { SectionRegistry } from './registry';

vi.mock('react-i18next', () => ({
  useTranslation: () => ({ t: (key: string) => key }),
}));

const registry: SectionRegistry = {
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

describe('LayoutRenderer', () => {
  it('renders sections in resolved order with mode passed through', () => {
    render(
      <LayoutRenderer
        registry={registry}
        layout={layoutOf([
          { type: 'beta.v1', mode: 'grid', presentation: 'visible' },
          { type: 'alpha.v1', presentation: 'visible' },
        ])}
      />
    );
    const texts = screen.getAllByText(/ALPHA|BETA/).map((n) => n.textContent);
    expect(texts).toEqual(['BETA:grid', 'ALPHA']);
  });

  it('renders a placeholder for presentation=placeholder and no component output', () => {
    render(
      <LayoutRenderer
        registry={registry}
        layout={layoutOf([{ type: 'alpha.v1', presentation: 'placeholder' }])}
      />
    );
    expect(screen.queryByText('ALPHA')).toBeNull();
    expect(screen.getByRole('status')).toHaveTextContent('layout.placeholderTitle');
  });

  it('skips unknown section types entirely and warns once per type', () => {
    const warn = vi.spyOn(console, 'warn').mockImplementation(() => {});
    render(
      <LayoutRenderer
        registry={registry}
        layout={layoutOf([
          { type: 'ghost.v9', presentation: 'visible' },
          { type: 'ghost.v9', presentation: 'visible' },
          { type: 'alpha.v1', presentation: 'visible' },
        ])}
      />
    );
    expect(screen.getByText('ALPHA')).toBeInTheDocument();
    expect(warn.mock.calls.filter((c) => String(c[0]).includes('ghost.v9'))).toHaveLength(1);
    warn.mockRestore();
  });

  it('isolates a crashing section: placeholder for it, siblings render', () => {
    const err = vi.spyOn(console, 'error').mockImplementation(() => {});
    render(
      <LayoutRenderer
        registry={registry}
        layout={layoutOf([
          { type: 'boom.v1', presentation: 'visible' },
          { type: 'alpha.v1', presentation: 'visible' },
        ])}
      />
    );
    expect(screen.getByText('ALPHA')).toBeInTheDocument();
    expect(screen.getByRole('status')).toBeInTheDocument();
    err.mockRestore();
  });

  it('skips malformed elements (null / non-object / missing string type) without throwing', () => {
    const malformed = [
      null,
      42,
      'string',
      { presentation: 'visible' },
      { type: 7, presentation: 'visible' },
      { type: 'alpha.v1', presentation: 'visible' },
    ] as unknown as ResolvedScreen['sections'];
    render(<LayoutRenderer registry={registry} layout={layoutOf(malformed)} />);
    expect(screen.getByText('ALPHA')).toBeInTheDocument();
    // Only the well-formed section rendered a wrapper
    expect(document.querySelectorAll('[data-layout-section]')).toHaveLength(1);
  });

  // -------------------------------------------------------------------------
  // data-layout-section tagging
  // -------------------------------------------------------------------------
  describe('data-layout-section tagging', () => {
    it('wraps every visible section in a div with data-layout-section={type}', () => {
      const { container } = render(
        <LayoutRenderer
          registry={registry}
          layout={layoutOf([
            { type: 'alpha.v1', presentation: 'visible' },
            { type: 'beta.v1', mode: 'grid', presentation: 'visible' },
          ])}
        />
      );
      expect(container.querySelector('[data-layout-section="alpha.v1"]')).toBeInTheDocument();
      expect(container.querySelector('[data-layout-section="beta.v1"]')).toBeInTheDocument();
    });

    it('wraps placeholder presentation rows in a div with data-layout-section={type}', () => {
      const { container } = render(
        <LayoutRenderer
          registry={registry}
          layout={layoutOf([{ type: 'alpha.v1', presentation: 'placeholder' }])}
        />
      );
      expect(container.querySelector('[data-layout-section="alpha.v1"]')).toBeInTheDocument();
    });
  });

  // -------------------------------------------------------------------------
  // onSectionClick prop
  // -------------------------------------------------------------------------
  describe('onSectionClick prop', () => {
    it('fires onSectionClick with the section type when clicking inside a section', async () => {
      const onSectionClick = vi.fn();
      const user = userEvent.setup();
      render(
        <LayoutRenderer
          registry={registry}
          layout={layoutOf([
            { type: 'alpha.v1', presentation: 'visible' },
            { type: 'beta.v1', mode: 'grid', presentation: 'visible' },
          ])}
          onSectionClick={onSectionClick}
        />
      );
      await user.click(screen.getByText('ALPHA'));
      expect(onSectionClick).toHaveBeenCalledWith('alpha.v1');
    });

    it('prevents default and stops propagation when onSectionClick is set', async () => {
      const onSectionClick = vi.fn();
      const outerClick = vi.fn();
      const user = userEvent.setup();
      const { container } = render(
        <div onClick={outerClick}>
          <LayoutRenderer
            registry={registry}
            layout={layoutOf([{ type: 'alpha.v1', presentation: 'visible' }])}
            onSectionClick={onSectionClick}
          />
        </div>
      );
      const wrapper = container.querySelector('[data-layout-section="alpha.v1"]');
      await user.click(wrapper!);
      expect(onSectionClick).toHaveBeenCalledWith('alpha.v1');
      // stopPropagation prevents the outer click handler from firing
      expect(outerClick).not.toHaveBeenCalled();
    });

    it('passes clicks through normally when onSectionClick is not set', async () => {
      const outerClick = vi.fn();
      const user = userEvent.setup();
      const { container } = render(
        <div onClick={outerClick}>
          <LayoutRenderer
            registry={registry}
            layout={layoutOf([{ type: 'alpha.v1', presentation: 'visible' }])}
          />
        </div>
      );
      const wrapper = container.querySelector('[data-layout-section="alpha.v1"]');
      await user.click(wrapper!);
      expect(outerClick).toHaveBeenCalled();
    });
  });
});
