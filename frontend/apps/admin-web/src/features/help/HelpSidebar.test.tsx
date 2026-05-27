/**
 * HelpSidebar component tests.
 *
 * Covers:
 *   - Rendering (article title, body, docs link)
 *   - Accessibility: role=dialog, aria-modal, aria-labelledby
 *   - Focus trap: useFocusTrap is invoked with the panel ref
 *   - Escape key closes the sidebar
 *   - Backdrop click closes the sidebar
 *   - Close button click closes the sidebar
 *   - No docs link when article.docsUrl is absent
 */

import { act, fireEvent, render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

// ---------------------------------------------------------------------------
// Mocks
// ---------------------------------------------------------------------------

vi.mock('react-i18next', () => ({
  useTranslation: () => ({
    t: (key: string, fallback?: string) => fallback ?? key,
  }),
}));

// Mock useFocusTrap: capture the onEscape callback so tests can invoke it,
// and simulate Tab-focus cycling is delegated to the hook (not re-tested here).
const capturedEscapeHandlers: Array<() => void> = [];

vi.mock('../../components/useFocusTrap', () => ({
  useFocusTrap: (_ref: unknown, onEscape: () => void) => {
    capturedEscapeHandlers.push(onEscape);
  },
}));

import type { HelpArticle } from '../../help/articles';
import { HelpSidebar } from './HelpSidebar';

// ---------------------------------------------------------------------------
// Fixtures
// ---------------------------------------------------------------------------

const ARTICLE_WITH_DOCS: HelpArticle = {
  id: 'test-article',
  title: 'Test Article',
  body: 'Simple body text.\n\n- List item one\n- List item two',
  docsUrl: '/docs/test',
};

const ARTICLE_NO_DOCS: HelpArticle = {
  id: 'no-docs-article',
  title: 'No Docs',
  body: 'No external link.',
};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function renderSidebar(props: Partial<{ article: HelpArticle; onClose: () => void }> = {}) {
  const onClose = props.onClose ?? vi.fn();
  const article = props.article ?? ARTICLE_WITH_DOCS;
  render(<HelpSidebar article={article} onClose={onClose} />);
  return { onClose };
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

beforeEach(() => {
  capturedEscapeHandlers.length = 0;
});

afterEach(() => {
  vi.clearAllMocks();
});

describe('HelpSidebar', () => {
  // --- Rendering ---

  it('renders the article title', () => {
    renderSidebar();
    expect(screen.getByText('Test Article')).toBeDefined();
  });

  it('renders article body as paragraphs', () => {
    renderSidebar();
    expect(screen.getByText('Simple body text.')).toBeDefined();
  });

  it('renders list items from body', () => {
    renderSidebar();
    expect(screen.getByText('List item one')).toBeDefined();
    expect(screen.getByText('List item two')).toBeDefined();
  });

  it('renders docs link when docsUrl is present', () => {
    renderSidebar({ article: ARTICLE_WITH_DOCS });
    const link = screen.getByRole('link');
    expect(link.getAttribute('href')).toBe('/docs/test');
  });

  it('does not render docs link when docsUrl is absent', () => {
    renderSidebar({ article: ARTICLE_NO_DOCS });
    expect(screen.queryByRole('link')).toBeNull();
  });

  // --- Accessibility: dialog semantics ---

  it('renders the panel with role="dialog"', () => {
    renderSidebar();
    expect(screen.getByRole('dialog')).toBeDefined();
  });

  it('panel has aria-modal="true"', () => {
    renderSidebar();
    const dialog = screen.getByRole('dialog');
    expect(dialog.getAttribute('aria-modal')).toBe('true');
  });

  it('panel has aria-labelledby that matches the title heading id', () => {
    renderSidebar();
    const dialog = screen.getByRole('dialog');
    const labelledBy = dialog.getAttribute('aria-labelledby');
    expect(labelledBy).toBeTruthy();

    // The heading with that id should contain the article title
    const heading = document.getElementById(labelledBy as string);
    expect(heading).not.toBeNull();
    expect(heading?.textContent).toContain('Test Article');
  });

  it('panel has tabIndex=-1 so useFocusTrap can programmatically focus it', () => {
    renderSidebar();
    const dialog = screen.getByRole('dialog');
    expect(Number(dialog.getAttribute('tabindex'))).toBe(-1);
  });

  // --- Close button ---

  it('renders a close button with accessible label', () => {
    renderSidebar();
    expect(screen.getByRole('button', { name: /close help/i })).toBeDefined();
  });

  it('calls onClose when close button is clicked', async () => {
    const user = userEvent.setup();
    const { onClose } = renderSidebar();
    await user.click(screen.getByRole('button', { name: /close help/i }));
    expect(onClose).toHaveBeenCalledTimes(1);
  });

  // --- Backdrop ---

  it('renders backdrop element', () => {
    const { container } = render(<HelpSidebar article={ARTICLE_WITH_DOCS} onClose={vi.fn()} />);
    expect(container.querySelector('.ppt-help-backdrop')).not.toBeNull();
  });

  it('calls onClose when backdrop is clicked', () => {
    const onClose = vi.fn();
    const { container } = render(<HelpSidebar article={ARTICLE_WITH_DOCS} onClose={onClose} />);
    const backdrop = container.querySelector('.ppt-help-backdrop') as HTMLElement;
    fireEvent.click(backdrop);
    expect(onClose).toHaveBeenCalledTimes(1);
  });

  // --- Escape key via useFocusTrap ---

  it('invokes useFocusTrap with onClose as the escape handler', () => {
    const onClose = vi.fn();
    renderSidebar({ onClose });
    // The mock captured the onEscape callback — invoking it should call onClose
    expect(capturedEscapeHandlers.length).toBeGreaterThan(0);
    act(() => {
      capturedEscapeHandlers[capturedEscapeHandlers.length - 1]();
    });
    expect(onClose).toHaveBeenCalledTimes(1);
  });

  // --- Styles injection guard ---

  it('does not inject duplicate style element on re-render', () => {
    const { rerender } = render(<HelpSidebar article={ARTICLE_WITH_DOCS} onClose={vi.fn()} />);
    rerender(<HelpSidebar article={ARTICLE_NO_DOCS} onClose={vi.fn()} />);

    const styleElements = document.querySelectorAll('#ppt-help-sidebar-styles');
    expect(styleElements.length).toBe(1);
  });
});
