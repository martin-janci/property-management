import { type ApiFormSummary, parseFormSummaries, toResidentForm } from './FormsScreen';

let warnSpy: jest.SpyInstance;
beforeEach(() => {
  warnSpy = jest.spyOn(console, 'warn').mockImplementation(() => {});
});
afterEach(() => {
  warnSpy.mockRestore();
});

const validForm: ApiFormSummary = {
  id: 'f-1',
  title: 'Fire-safety declaration',
  description: 'Confirm equipment is functional.',
  category: 'compliance',
  status: 'published',
  require_signatures: true,
  submission_deadline: '2026-04-30T23:59:00Z',
};

describe('parseFormSummaries', () => {
  it('accepts the wrapped `{ forms: [...] }` envelope', () => {
    expect(parseFormSummaries({ forms: [validForm] })).toEqual([validForm]);
  });

  it('accepts a bare array (defensive)', () => {
    expect(parseFormSummaries([validForm])).toEqual([validForm]);
  });

  it.each([
    ['null', null],
    ['undefined', undefined],
    ['a number', 42],
    ['a string', 'nope'],
    ['an object without forms', { error: 'boom' }],
    ['an object whose forms is not an array', { forms: 'nope' }],
  ])('returns [] for an unexpected top-level shape: %s', (_label, input) => {
    expect(parseFormSummaries(input)).toEqual([]);
  });

  it('drops malformed entries instead of throwing', () => {
    const mixed = [validForm, null, 'garbage', { id: 'no-title' }, { title: 'no-id' }];
    expect(parseFormSummaries({ forms: mixed })).toEqual([validForm]);
  });

  it('emits a dev-only warning for an unrecognized non-null payload', () => {
    parseFormSummaries({ error: 'boom' });
    expect(warnSpy).toHaveBeenCalledWith(expect.stringContaining('parseFormSummaries'));
  });

  it('emits a dev-only warning when every candidate row is dropped', () => {
    parseFormSummaries({ forms: [{ noId: true }] });
    expect(warnSpy).toHaveBeenCalledWith(expect.stringContaining('dropped all 1'));
  });

  it('stays silent for a legitimately empty response', () => {
    parseFormSummaries({ forms: [] });
    parseFormSummaries(null);
    expect(warnSpy).not.toHaveBeenCalled();
  });
});

describe('toResidentForm', () => {
  it('maps the api summary onto the UI form shape (no per-resident status — see #2129)', () => {
    expect(toResidentForm(validForm)).toEqual({
      id: 'f-1',
      title: 'Fire-safety declaration',
      description: 'Confirm equipment is functional.',
      dueAt: '2026-04-30T23:59:00Z',
      required: true,
    });
  });

  it('defaults optional fields', () => {
    const mapped = toResidentForm({ id: 'f-2', title: 'Survey' });
    expect(mapped.description).toBe('');
    expect(mapped.dueAt).toBeUndefined();
    expect(mapped.required).toBe(false);
  });
});
