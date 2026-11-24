import { describe, expect, it } from 'vitest';
import { changedFlags, flagFields, flagInitialValues } from './featureFlagForm';

const flags = [
  { id: '2', key: 'new_dashboard', name: 'New dashboard', is_enabled: true },
  { id: '1', key: 'dark_mode', is_enabled: false },
];

describe('feature flag form model', () => {
  it('builds one field per server flag, sorted by key', () => {
    expect(flagFields(flags)).toEqual([
      { kind: 'boolean', key: 'dark_mode', label: 'dark_mode' },
      { kind: 'boolean', key: 'new_dashboard', label: 'New dashboard (new_dashboard)' },
    ]);
    expect(flagFields([])).toEqual([]);
  });

  it('takes initial values from the server state', () => {
    expect(flagInitialValues(flags)).toEqual({ new_dashboard: true, dark_mode: false });
  });

  it('returns only the flags that actually changed', () => {
    expect(changedFlags(flags, { new_dashboard: true, dark_mode: false })).toEqual([]);
    expect(
      changedFlags(flags, { new_dashboard: false, dark_mode: false }).map((f) => f.id)
    ).toEqual(['2']);
    expect(changedFlags(flags, { new_dashboard: true, dark_mode: true }).map((f) => f.id)).toEqual([
      '1',
    ]);
  });
});
