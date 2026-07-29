import type { BaseSection, LayoutManifest, LayoutRails, TenantOverride } from '@ppt/api-client';
import { fireEvent, render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';
import { TenantSectionEditor } from './TenantSectionEditor';

vi.mock('react-i18next', () => ({
  useTranslation: () => ({ t: (key: string) => key }),
}));

// ---------------------------------------------------------------------------
// Fixtures
// ---------------------------------------------------------------------------

const base: BaseSection[] = [
  { type: 'hero', visible: true },
  { type: 'stats', visible: true },
  { type: 'feed', visible: true },
];

const noRails: LayoutRails = {
  hideable: [],
  mode_editable: [],
  reorderable: false,
  prop_whitelist: {},
};

const hideableRails: LayoutRails = {
  ...noRails,
  hideable: ['hero'],
};

const reorderableRails: LayoutRails = {
  ...noRails,
  reorderable: true,
};

const modeRails: LayoutRails = {
  ...noRails,
  mode_editable: ['feed'],
};

const feedManifest: LayoutManifest = {
  platform: 'web',
  components: {
    feed: { supported_modes: ['list', 'grid'], default_mode: 'list' },
  },
};

const propRails: LayoutRails = {
  ...noRails,
  prop_whitelist: { hero: ['limit'] },
};

const emptyOverride: TenantOverride = {};

// ---------------------------------------------------------------------------
// Scenario 1: hideable-only section
// ---------------------------------------------------------------------------

describe('Scenario 1: hideable-only section', () => {
  it('shows eye button; no arrows, no mode select, no prop input', () => {
    render(
      <TenantSectionEditor
        baseSections={[{ type: 'hero', visible: true }]}
        rails={hideableRails}
        manifest={null}
        override={emptyOverride}
        onChange={vi.fn()}
      />
    );
    expect(
      screen.getByRole('button', { name: /layout\.customize\.(hide|show)/ })
    ).toBeInTheDocument();
    expect(screen.queryByRole('button', { name: /layout\.customize\.moveUp/ })).toBeNull();
    expect(screen.queryByRole('button', { name: /layout\.customize\.moveDown/ })).toBeNull();
    expect(screen.queryByRole('combobox')).toBeNull();
    expect(screen.queryByRole('textbox')).toBeNull();
  });

  it('clicking eye calls onChange with visible patch', async () => {
    const user = userEvent.setup();
    const onChange = vi.fn();
    render(
      <TenantSectionEditor
        baseSections={[{ type: 'hero', visible: true }]}
        rails={hideableRails}
        manifest={null}
        override={emptyOverride}
        onChange={onChange}
      />
    );
    await user.click(screen.getByRole('button', { name: /layout\.customize\.(hide|show)/ }));
    expect(onChange).toHaveBeenCalledOnce();
    const next = onChange.mock.calls[0][0] as TenantOverride;
    expect(next.sections?.hero?.visible).toBe(false);
  });
});

// ---------------------------------------------------------------------------
// Scenario 2: non-granted section — no interactive controls
// ---------------------------------------------------------------------------

describe('Scenario 2: non-granted section', () => {
  it('renders no interactive controls', () => {
    render(
      <TenantSectionEditor
        baseSections={[{ type: 'hero' }]}
        rails={noRails}
        manifest={null}
        override={emptyOverride}
        onChange={vi.fn()}
      />
    );
    expect(screen.queryByRole('button')).toBeNull();
    expect(screen.queryByRole('combobox')).toBeNull();
    expect(screen.queryByRole('textbox')).toBeNull();
  });
});

// ---------------------------------------------------------------------------
// Scenario 3: reorderable rails — up/down arrows; clicking ↓ writes full order
// ---------------------------------------------------------------------------

describe('Scenario 3: reorderable rails', () => {
  it('renders up and down arrow buttons for each row', () => {
    render(
      <TenantSectionEditor
        baseSections={base}
        rails={reorderableRails}
        manifest={null}
        override={emptyOverride}
        onChange={vi.fn()}
      />
    );
    expect(screen.getAllByRole('button', { name: 'layout.customize.moveUp' })).toHaveLength(3);
    expect(screen.getAllByRole('button', { name: 'layout.customize.moveDown' })).toHaveLength(3);
  });

  it('clicking ↓ on first row writes full updated order array', async () => {
    const user = userEvent.setup();
    const onChange = vi.fn();
    render(
      <TenantSectionEditor
        baseSections={base}
        rails={reorderableRails}
        manifest={null}
        override={emptyOverride}
        onChange={onChange}
      />
    );
    const downButtons = screen.getAllByRole('button', { name: 'layout.customize.moveDown' });
    await user.click(downButtons[0]);
    const next = onChange.mock.calls[0][0] as TenantOverride;
    expect(next.order).toEqual(['stats', 'hero', 'feed']);
  });
});

// ---------------------------------------------------------------------------
// Scenario 4: mode select
// ---------------------------------------------------------------------------

describe('Scenario 4: mode select', () => {
  it('renders select only for type in mode_editable with supported_modes in manifest', () => {
    render(
      <TenantSectionEditor
        baseSections={[{ type: 'feed', mode: 'list' }]}
        rails={modeRails}
        manifest={feedManifest}
        override={emptyOverride}
        onChange={vi.fn()}
      />
    );
    expect(screen.getByRole('combobox', { name: 'layout.customize.mode' })).toBeInTheDocument();
  });

  it('does not render select when manifest has no supported_modes', () => {
    const manifestNoModes: LayoutManifest = {
      platform: 'web',
      components: { feed: {} },
    };
    render(
      <TenantSectionEditor
        baseSections={[{ type: 'feed' }]}
        rails={modeRails}
        manifest={manifestNoModes}
        override={emptyOverride}
        onChange={vi.fn()}
      />
    );
    expect(screen.queryByRole('combobox')).toBeNull();
  });

  it('changing select patches mode in onChange', async () => {
    const user = userEvent.setup();
    const onChange = vi.fn();
    render(
      <TenantSectionEditor
        baseSections={[{ type: 'feed', mode: 'list' }]}
        rails={modeRails}
        manifest={feedManifest}
        override={emptyOverride}
        onChange={onChange}
      />
    );
    await user.selectOptions(
      screen.getByRole('combobox', { name: 'layout.customize.mode' }),
      'grid'
    );
    expect(onChange).toHaveBeenCalledOnce();
    const next = onChange.mock.calls[0][0] as TenantOverride;
    expect(next.sections?.feed?.mode).toBe('grid');
  });

  it('renders an empty "default" option first and shows it when no override mode is set', () => {
    render(
      <TenantSectionEditor
        baseSections={[{ type: 'feed', mode: 'list' }]}
        rails={modeRails}
        manifest={feedManifest}
        override={emptyOverride}
        onChange={vi.fn()}
      />
    );
    const select = screen.getByRole('combobox', {
      name: 'layout.customize.mode',
    }) as HTMLSelectElement;
    // First option is the empty "default" option, selected when patch has no mode
    expect(select.options[0].value).toBe('');
    expect(select.value).toBe('');
  });

  it('selecting the empty option removes the mode from the patch', async () => {
    const user = userEvent.setup();
    const onChange = vi.fn();
    render(
      <TenantSectionEditor
        baseSections={[{ type: 'feed', mode: 'list' }]}
        rails={modeRails}
        manifest={feedManifest}
        override={{ sections: { feed: { mode: 'grid' } } }}
        onChange={onChange}
      />
    );
    await user.selectOptions(screen.getByRole('combobox', { name: 'layout.customize.mode' }), '');
    expect(onChange).toHaveBeenCalledOnce();
    const next = onChange.mock.calls[0][0] as TenantOverride;
    // mode removed → patch pruned entirely
    expect(next.sections?.feed).toBeUndefined();
  });

  it('does not throw on a malformed manifest missing the components map', () => {
    const malformed = { platform: 'web' } as unknown as LayoutManifest;
    expect(() =>
      render(
        <TenantSectionEditor
          baseSections={[{ type: 'feed', mode: 'list' }]}
          rails={modeRails}
          manifest={malformed}
          override={emptyOverride}
          onChange={vi.fn()}
        />
      )
    ).not.toThrow();
    // No supported modes → no select
    expect(screen.queryByRole('combobox')).toBeNull();
  });
});

// ---------------------------------------------------------------------------
// Scenario 5: whitelisted prop input
// ---------------------------------------------------------------------------

describe('Scenario 5: whitelisted prop input', () => {
  it('commits JSON number when value is parseable', async () => {
    const user = userEvent.setup();
    const onChange = vi.fn();
    render(
      <TenantSectionEditor
        baseSections={[{ type: 'hero' }]}
        rails={propRails}
        manifest={null}
        override={emptyOverride}
        onChange={onChange}
      />
    );
    const input = screen.getByRole('textbox', { name: 'limit' });
    await user.click(input);
    await user.type(input, '5');
    await user.tab(); // blur
    expect(onChange).toHaveBeenCalledOnce();
    const next = onChange.mock.calls[0][0] as TenantOverride;
    expect(next.sections?.hero?.props?.limit).toBe(5);
  });

  it('commits plain string when value is not valid JSON', async () => {
    const user = userEvent.setup();
    const onChange = vi.fn();
    render(
      <TenantSectionEditor
        baseSections={[{ type: 'hero' }]}
        rails={propRails}
        manifest={null}
        override={emptyOverride}
        onChange={onChange}
      />
    );
    const input = screen.getByRole('textbox', { name: 'limit' });
    await user.click(input);
    await user.type(input, 'many');
    await user.tab();
    const next = onChange.mock.calls[0][0] as TenantOverride;
    expect(next.sections?.hero?.props?.limit).toBe('many');
  });

  it('clearing the input removes the prop and prunes empty objects', async () => {
    const user = userEvent.setup();
    const onChange = vi.fn();
    const overrideWithProp: TenantOverride = {
      sections: { hero: { props: { limit: 10 } } },
    };
    render(
      <TenantSectionEditor
        baseSections={[{ type: 'hero' }]}
        rails={propRails}
        manifest={null}
        override={overrideWithProp}
        onChange={onChange}
      />
    );
    const input = screen.getByRole('textbox', { name: 'limit' });
    await user.clear(input);
    await user.tab();
    const next = onChange.mock.calls[0][0] as TenantOverride;
    // props pruned → sections.hero has no props → sections pruned entirely
    expect(next.sections?.hero?.props).toBeUndefined();
    expect(next.sections?.hero).toBeUndefined();
  });
});

// ---------------------------------------------------------------------------
// Scenario 5a: string-typed prop is NOT JSON-coerced on commit
// ---------------------------------------------------------------------------

describe('Scenario 5a: string-typed prop keeps string values', () => {
  // Whitelist a prop whose base (declared) value is a string. The declared
  // type must gate coercion so JSON-looking input stays a string instead of
  // being silently parsed into a boolean / array / number.
  const stringPropRails: LayoutRails = {
    ...noRails,
    prop_whitelist: { hero: ['label'] },
  };
  const stringBase: BaseSection[] = [{ type: 'hero', props: { label: 'Welcome' } }];

  // fireEvent.change is used instead of userEvent.type because the JSON-ish
  // inputs ('[]') contain characters userEvent parses as keyboard descriptors.
  it.each([['true'], ['[]'], ['123']])('input %j round-trips as the string %j', async (typed) => {
    const onChange = vi.fn();
    render(
      <TenantSectionEditor
        baseSections={stringBase}
        rails={stringPropRails}
        manifest={null}
        override={emptyOverride}
        onChange={onChange}
      />
    );
    const input = screen.getByRole('textbox', { name: 'label' });
    fireEvent.change(input, { target: { value: typed } });
    fireEvent.blur(input); // commit
    expect(onChange).toHaveBeenCalledOnce();
    const next = onChange.mock.calls[0][0] as TenantOverride;
    const committed = next.sections?.hero?.props?.label;
    expect(committed).toBe(typed);
    expect(typeof committed).toBe('string');
  });
});

// ---------------------------------------------------------------------------
// Scenario 5b: external override reset remounts PropInput (carried finding)
// ---------------------------------------------------------------------------

describe('Scenario 5b: external reset clears PropInput value', () => {
  it('shows empty input after override is reset to {} via rerender', () => {
    const { rerender } = render(
      <TenantSectionEditor
        baseSections={[{ type: 'hero' }]}
        rails={propRails}
        manifest={null}
        override={{ sections: { hero: { props: { limit: 42 } } } }}
        onChange={vi.fn()}
      />
    );

    // Confirm initial value shown
    const inputBefore = screen.getByRole('textbox', { name: 'limit' }) as HTMLInputElement;
    expect(inputBefore.value).toBe('42');

    // External reset: override → {}
    rerender(
      <TenantSectionEditor
        baseSections={[{ type: 'hero' }]}
        rails={propRails}
        manifest={null}
        override={{}}
        onChange={vi.fn()}
      />
    );

    // After remount the input should show placeholder/empty (no override value)
    const inputAfter = screen.getByRole('textbox', { name: 'limit' }) as HTMLInputElement;
    expect(inputAfter.value).toBe('');
  });
});

// ---------------------------------------------------------------------------
// Scenario 6: override.order drives row order
// ---------------------------------------------------------------------------

describe('Scenario 6: override.order drives row order', () => {
  it('listed sections appear first in override.order order; unlisted keep base order', () => {
    const override: TenantOverride = { order: ['feed', 'hero'] };
    render(
      <TenantSectionEditor
        baseSections={base}
        rails={noRails}
        manifest={null}
        override={override}
        onChange={vi.fn()}
      />
    );
    const rows = screen.getAllByText(/^(hero|stats|feed)$/);
    const types = rows.map((el) => el.textContent);
    // feed, hero listed; stats unlisted → comes after
    expect(types).toEqual(['feed', 'hero', 'stats']);
  });
});
