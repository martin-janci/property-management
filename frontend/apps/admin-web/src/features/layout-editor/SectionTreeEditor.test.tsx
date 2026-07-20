/**
 * SectionTreeEditor — TDD test suite (Task 2)
 *
 * 7 scenarios covering the full behaviour contract.
 *
 * react-i18next is mocked so t() returns defaultValue (or the key if none).
 * window.confirm is mocked per test that needs it.
 */

import { fireEvent, render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';
import type { Manifest, SectionConfig } from './api';
import { SectionTreeEditor } from './SectionTreeEditor';

// ---------------------------------------------------------------------------
// Mock react-i18next
// ---------------------------------------------------------------------------
vi.mock('react-i18next', () => ({
  useTranslation: () => ({
    t: (_k: string, o?: { defaultValue?: string }) => o?.defaultValue ?? _k,
  }),
  Trans: ({ children }: { children: React.ReactNode }) => children,
}));

// ---------------------------------------------------------------------------
// Fixture data
// ---------------------------------------------------------------------------

const MANIFEST: Manifest = {
  platform: 'web',
  components: {
    'gallery.v1': { required: true },
    'faq.v1': { supported_modes: ['accordion', 'list'], default_mode: 'accordion' },
    'promo.v1': {},
  },
};

const SECTIONS: SectionConfig[] = [
  { type: 'gallery.v1', visible: true },
  { type: 'faq.v1', visible: true },
];

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function buildProps(overrides: Partial<Parameters<typeof SectionTreeEditor>[0]> = {}) {
  return {
    sections: SECTIONS,
    manifest: MANIFEST,
    kills: [] as string[],
    onChange: vi.fn(),
    onKill: vi.fn(),
    onUnkill: vi.fn(),
    ...overrides,
  };
}

// ---------------------------------------------------------------------------
// Scenario 1 — required lock badge / no hide button; optional eye toggle
// ---------------------------------------------------------------------------

describe('Scenario 1: required vs optional sections', () => {
  it('shows lock badge for required section and NO hide button', () => {
    render(<SectionTreeEditor {...buildProps()} />);

    // gallery.v1 is required → lock badge present
    expect(screen.getByTestId('lock-badge-gallery.v1')).toBeTruthy();

    // no hide/eye control at all for the required section row
    expect(screen.queryByTestId('hide-btn-gallery.v1')).toBeNull();
  });

  it('shows eye toggle for optional section; clicking calls onChange with visible:false', async () => {
    const onChange = vi.fn();
    const user = userEvent.setup();
    render(<SectionTreeEditor {...buildProps({ onChange })} />);

    const eyeBtn = screen.getByTestId('hide-btn-faq.v1');
    await user.click(eyeBtn);

    expect(onChange).toHaveBeenCalledOnce();
    const next: SectionConfig[] = onChange.mock.calls[0][0];
    const faqRow = next.find((s) => s.type === 'faq.v1');
    expect(faqRow?.visible).toBe(false);
  });
});

// ---------------------------------------------------------------------------
// Scenario 2 — reorder ↑/↓; disabled at edges
// ---------------------------------------------------------------------------

describe('Scenario 2: reorder via ↑/↓ buttons', () => {
  it('↓ on first section calls onChange with swapped order', async () => {
    const onChange = vi.fn();
    const user = userEvent.setup();
    render(<SectionTreeEditor {...buildProps({ onChange })} />);

    const downBtn = screen.getByTestId('move-down-gallery.v1');
    await user.click(downBtn);

    expect(onChange).toHaveBeenCalledOnce();
    const next: SectionConfig[] = onChange.mock.calls[0][0];
    expect(next[0].type).toBe('faq.v1');
    expect(next[1].type).toBe('gallery.v1');
  });

  it('↑ is disabled on first section, ↓ is disabled on last section', () => {
    render(<SectionTreeEditor {...buildProps()} />);

    expect((screen.getByTestId('move-up-gallery.v1') as HTMLButtonElement).disabled).toBe(true);
    expect((screen.getByTestId('move-down-faq.v1') as HTMLButtonElement).disabled).toBe(true);
  });
});

// ---------------------------------------------------------------------------
// Scenario 3 — mode select
// ---------------------------------------------------------------------------

describe('Scenario 3: mode select', () => {
  it('renders mode select only for sections with supported_modes', () => {
    render(<SectionTreeEditor {...buildProps()} />);

    // gallery.v1 has no supported_modes → no select
    expect(screen.queryByTestId('mode-select-gallery.v1')).toBeNull();

    // faq.v1 has supported_modes → select present with the manifest options
    const select = screen.getByTestId('mode-select-faq.v1') as HTMLSelectElement;
    expect(select).toBeTruthy();
    const options = Array.from(select.options).map((o) => o.value);
    expect(options).toContain('accordion');
    expect(options).toContain('list');
  });

  it('changing mode select calls onChange with patched mode', async () => {
    const onChange = vi.fn();
    const user = userEvent.setup();
    render(<SectionTreeEditor {...buildProps({ onChange })} />);

    const select = screen.getByTestId('mode-select-faq.v1');
    await user.selectOptions(select, 'list');

    expect(onChange).toHaveBeenCalledOnce();
    const next: SectionConfig[] = onChange.mock.calls[0][0];
    const faqRow = next.find((s) => s.type === 'faq.v1');
    expect(faqRow?.mode).toBe('list');
  });
});

// ---------------------------------------------------------------------------
// Scenario 4 — props textarea JSON editing
// ---------------------------------------------------------------------------

describe('Scenario 4: props textarea', () => {
  it('invalid JSON shows inline error and does NOT call onChange', () => {
    const onChange = vi.fn();
    render(<SectionTreeEditor {...buildProps({ onChange })} />);

    const textarea = screen.getByTestId('props-textarea-faq.v1');
    // fireEvent.change avoids user-event's curly-brace special-char parsing
    fireEvent.change(textarea, { target: { value: 'NOT JSON {{{' } });
    fireEvent.blur(textarea);

    expect(screen.getByTestId('props-error-faq.v1')).toBeTruthy();
    expect(onChange).not.toHaveBeenCalled();
  });

  it('valid JSON calls onChange with parsed props', () => {
    const onChange = vi.fn();
    render(<SectionTreeEditor {...buildProps({ onChange })} />);

    const textarea = screen.getByTestId('props-textarea-faq.v1');
    fireEvent.change(textarea, { target: { value: '{"foo":"bar"}' } });
    fireEvent.blur(textarea);

    expect(screen.queryByTestId('props-error-faq.v1')).toBeNull();
    expect(onChange).toHaveBeenCalledOnce();
    const next: SectionConfig[] = onChange.mock.calls[0][0];
    const faqRow = next.find((s) => s.type === 'faq.v1');
    expect(faqRow?.props).toEqual({ foo: 'bar' });
  });
});

// ---------------------------------------------------------------------------
// Scenario 5 — killed badge + kill/unkill
// ---------------------------------------------------------------------------

describe('Scenario 5: killed badge and kill/unkill', () => {
  it('killed section shows killed badge and Unkill button', () => {
    render(<SectionTreeEditor {...buildProps({ kills: ['faq.v1'] })} />);

    expect(screen.getByTestId('killed-badge-faq.v1')).toBeTruthy();
    expect(screen.getByTestId('unkill-btn-faq.v1')).toBeTruthy();
    expect(screen.queryByTestId('kill-btn-faq.v1')).toBeNull();
  });

  it('un-killed section shows Kill button (not killed badge)', () => {
    render(<SectionTreeEditor {...buildProps({ kills: [] })} />);

    expect(screen.queryByTestId('killed-badge-faq.v1')).toBeNull();
    expect(screen.getByTestId('kill-btn-faq.v1')).toBeTruthy();
  });

  it('clicking Kill (confirm mocked true) calls onKill with the type', async () => {
    const onKill = vi.fn();
    const user = userEvent.setup();
    vi.spyOn(window, 'confirm').mockReturnValue(true);

    render(<SectionTreeEditor {...buildProps({ onKill })} />);

    await user.click(screen.getByTestId('kill-btn-faq.v1'));

    expect(window.confirm).toHaveBeenCalledOnce();
    expect(onKill).toHaveBeenCalledWith('faq.v1');

    vi.restoreAllMocks();
  });

  it('clicking Kill (confirm mocked false) does NOT call onKill', async () => {
    const onKill = vi.fn();
    const user = userEvent.setup();
    vi.spyOn(window, 'confirm').mockReturnValue(false);

    render(<SectionTreeEditor {...buildProps({ onKill })} />);

    await user.click(screen.getByTestId('kill-btn-faq.v1'));

    expect(onKill).not.toHaveBeenCalled();

    vi.restoreAllMocks();
  });
});

// ---------------------------------------------------------------------------
// Scenario 6 — add-section select
// ---------------------------------------------------------------------------

describe('Scenario 6: add-section select', () => {
  it('lists only manifest types not already present in sections', () => {
    render(<SectionTreeEditor {...buildProps()} />);

    const addSelect = screen.getByTestId('add-section-select') as HTMLSelectElement;
    const options = Array.from(addSelect.options)
      .map((o) => o.value)
      .filter(Boolean);

    // promo.v1 is in manifest but not in sections
    expect(options).toContain('promo.v1');
    // gallery.v1 and faq.v1 are already present
    expect(options).not.toContain('gallery.v1');
    expect(options).not.toContain('faq.v1');
  });

  it('clicking Add appends and calls onChange', async () => {
    const onChange = vi.fn();
    const user = userEvent.setup();
    render(<SectionTreeEditor {...buildProps({ onChange })} />);

    const addSelect = screen.getByTestId('add-section-select');
    await user.selectOptions(addSelect, 'promo.v1');
    await user.click(screen.getByTestId('add-section-btn'));

    expect(onChange).toHaveBeenCalledOnce();
    const next: SectionConfig[] = onChange.mock.calls[0][0];
    expect(next[next.length - 1]).toMatchObject({ type: 'promo.v1', visible: true });
  });
});

// ---------------------------------------------------------------------------
// Scenario 5b — unkill confirm
// ---------------------------------------------------------------------------

describe('Scenario 5b: unkill confirm', () => {
  it('clicking Unkill (confirm mocked true) calls onUnkill with the type', async () => {
    const onUnkill = vi.fn();
    const user = userEvent.setup();
    vi.spyOn(window, 'confirm').mockReturnValue(true);

    render(<SectionTreeEditor {...buildProps({ kills: ['faq.v1'], onUnkill })} />);

    await user.click(screen.getByTestId('unkill-btn-faq.v1'));

    expect(window.confirm).toHaveBeenCalledOnce();
    expect(onUnkill).toHaveBeenCalledWith('faq.v1');

    vi.restoreAllMocks();
  });

  it('clicking Unkill (confirm mocked false) does NOT call onUnkill', async () => {
    const onUnkill = vi.fn();
    const user = userEvent.setup();
    vi.spyOn(window, 'confirm').mockReturnValue(false);

    render(<SectionTreeEditor {...buildProps({ kills: ['faq.v1'], onUnkill })} />);

    await user.click(screen.getByTestId('unkill-btn-faq.v1'));

    expect(onUnkill).not.toHaveBeenCalled();

    vi.restoreAllMocks();
  });
});

// ---------------------------------------------------------------------------
// Scenario 8 — props resync when sections prop changes externally
// ---------------------------------------------------------------------------

describe('Scenario 8: props textarea resyncs on external sections change', () => {
  it('shows updated props when sections prop changes (draft reload / rollback)', () => {
    const props = buildProps({
      sections: [{ type: 'faq.v1', visible: true, props: { color: 'red' } }],
    });
    const { rerender } = render(<SectionTreeEditor {...props} />);

    // Initially shows original props
    const textarea = screen.getByTestId('props-textarea-faq.v1') as HTMLTextAreaElement;
    expect(textarea.value).toContain('red');

    // Simulate external update (draft reload / rollback)
    rerender(
      <SectionTreeEditor
        {...props}
        sections={[{ type: 'faq.v1', visible: true, props: { color: 'blue' } }]}
      />
    );

    expect((screen.getByTestId('props-textarea-faq.v1') as HTMLTextAreaElement).value).toContain(
      'blue'
    );
  });

  it('adding a section then blurring its props textarea shows no error and no spurious onChange', () => {
    const onChange = vi.fn();

    // Start with sections that have a newly-added type (no props set)
    render(
      <SectionTreeEditor
        {...buildProps({
          onChange,
          sections: [
            { type: 'gallery.v1', visible: true },
            { type: 'faq.v1', visible: true },
            { type: 'promo.v1', visible: true },
          ],
        })}
      />
    );

    const textarea = screen.getByTestId('props-textarea-promo.v1');

    // Focus then blur without typing — derived text is '{}' which is valid JSON
    fireEvent.focus(textarea);
    fireEvent.blur(textarea);

    // No error shown
    expect(screen.queryByTestId('props-error-promo.v1')).toBeNull();
    // onChange called with valid empty props ({}), not suppressed
    expect(onChange).toHaveBeenCalledOnce();
    const next: SectionConfig[] = onChange.mock.calls[0][0];
    const promoRow = next.find((s) => s.type === 'promo.v1');
    expect(promoRow?.props).toEqual({});
  });
});

// ---------------------------------------------------------------------------
// Scenario 9 — duplicate section types (possible via rollback/server data)
// ---------------------------------------------------------------------------

describe('Scenario 9: duplicate section types are addressed per-row (by index)', () => {
  const DUP_SECTIONS: SectionConfig[] = [
    { type: 'faq.v1', visible: true, props: { slot: 'first' } },
    { type: 'faq.v1', visible: true, props: { slot: 'second' } },
  ];

  it('renders one row per entry even when types collide', () => {
    render(<SectionTreeEditor {...buildProps({ sections: DUP_SECTIONS })} />);
    expect(screen.getAllByTestId('props-textarea-faq.v1')).toHaveLength(2);
  });

  it('toggling visibility on the second copy only affects that copy', async () => {
    const onChange = vi.fn();
    const user = userEvent.setup();
    render(<SectionTreeEditor {...buildProps({ sections: DUP_SECTIONS, onChange })} />);

    const eyeButtons = screen.getAllByTestId('hide-btn-faq.v1');
    expect(eyeButtons).toHaveLength(2);
    await user.click(eyeButtons[1]);

    expect(onChange).toHaveBeenCalledOnce();
    const next: SectionConfig[] = onChange.mock.calls[0][0];
    expect(next[0].visible).toBe(true);
    expect(next[1].visible).toBe(false);
  });

  it('editing props on the first copy only affects that copy', () => {
    const onChange = vi.fn();
    render(<SectionTreeEditor {...buildProps({ sections: DUP_SECTIONS, onChange })} />);

    const textareas = screen.getAllByTestId('props-textarea-faq.v1');
    fireEvent.focus(textareas[0]);
    fireEvent.change(textareas[0], { target: { value: '{"slot":"edited"}' } });
    fireEvent.blur(textareas[0]);

    expect(onChange).toHaveBeenCalledOnce();
    const next: SectionConfig[] = onChange.mock.calls[0][0];
    expect(next[0].props).toEqual({ slot: 'edited' });
    expect(next[1].props).toEqual({ slot: 'second' });
  });

  it('removing the first copy keeps the second', async () => {
    const onChange = vi.fn();
    const user = userEvent.setup();
    render(<SectionTreeEditor {...buildProps({ sections: DUP_SECTIONS, onChange })} />);

    const removeButtons = screen.getAllByTestId('remove-btn-faq.v1');
    await user.click(removeButtons[0]);

    expect(onChange).toHaveBeenCalledOnce();
    const next: SectionConfig[] = onChange.mock.calls[0][0];
    expect(next).toHaveLength(1);
    expect(next[0].props).toEqual({ slot: 'second' });
  });
});

// ---------------------------------------------------------------------------
// Scenario 7 — unknown-type warning badge
// ---------------------------------------------------------------------------

describe('Scenario 7: unknown-type warning badge', () => {
  it('shows warning badge for a section whose type is not in the manifest', () => {
    const sectionsWithUnknown: SectionConfig[] = [
      ...SECTIONS,
      { type: 'mystery.v9', visible: true },
    ];
    render(<SectionTreeEditor {...buildProps({ sections: sectionsWithUnknown })} />);

    expect(screen.getByTestId('unknown-badge-mystery.v9')).toBeTruthy();
  });

  it('does NOT show warning badge for known types', () => {
    render(<SectionTreeEditor {...buildProps()} />);

    expect(screen.queryByTestId('unknown-badge-gallery.v1')).toBeNull();
    expect(screen.queryByTestId('unknown-badge-faq.v1')).toBeNull();
  });
});
