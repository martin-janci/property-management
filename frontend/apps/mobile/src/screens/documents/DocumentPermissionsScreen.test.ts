import { unwrapDocumentDetail } from './DocumentPermissionsScreen';

const doc = {
  id: 'd-1',
  title: 'House rules',
  access_scope: 'building',
  created_at: '2026-01-02T00:00:00Z',
  updated_at: '2026-01-03T00:00:00Z',
};

describe('unwrapDocumentDetail', () => {
  it('reads the document out of the `{ document }` envelope the server sends', () => {
    expect(unwrapDocumentDetail({ document: doc })).toEqual(doc);
  });

  it('accepts a flat payload', () => {
    expect(unwrapDocumentDetail(doc)).toEqual(doc);
  });

  it.each([null, undefined, 'x', {}, { document: null }, { document: {} }])(
    'returns undefined for %p',
    (value) => {
      expect(unwrapDocumentDetail(value)).toBeUndefined();
    }
  );
});
