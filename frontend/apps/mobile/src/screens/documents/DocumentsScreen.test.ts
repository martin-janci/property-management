import { type ApiDocument, pickDocumentType, toUiDocument } from './DocumentsScreen';

// Regression for PR #918 (High #4): DocumentsScreen previously mapped the
// legacy `name`/`file_path`/`content_type`/`uploaded_at` fields, but the
// backend `DocumentSummary` returns `title`/`file_name`/`mime_type`/
// `created_at`/`folder_id`. The wrong mapping left names blank, type icons
// generic, and folder grouping broken. These tests pin the corrected
// field mapping so the contract can't silently regress again.

const baseDoc: ApiDocument = {
  id: 'doc-1',
  title: 'Building Rules',
  category: 'general',
  file_name: 'rules.pdf',
  mime_type: 'application/pdf',
  size_bytes: 2048,
  folder_id: 'folder-9',
  created_at: '2026-05-01T10:00:00Z',
};

describe('toUiDocument (PR #918 field mapping)', () => {
  it('maps backend summary fields onto the UI Document shape', () => {
    expect(toUiDocument(baseDoc)).toEqual({
      id: 'doc-1',
      name: 'Building Rules', // from `title`, NOT legacy `name`
      type: 'pdf',
      size: 2048, // from `size_bytes`
      createdAt: '2026-05-01T10:00:00Z',
      updatedAt: '2026-05-01T10:00:00Z', // summary has no separate upload ts
      parentId: 'folder-9', // from `folder_id`
      downloadUrl: undefined, // not present on the summary
      children: undefined,
    });
  });

  it('uses `title` for the name even when it differs from file_name', () => {
    const ui = toUiDocument({ ...baseDoc, title: 'Q1 Budget', file_name: 'budget-q1.xlsx' });
    expect(ui.name).toBe('Q1 Budget');
  });

  it('defaults parentId to null when folder_id is absent', () => {
    const { folder_id, ...withoutFolder } = baseDoc;
    expect(toUiDocument(withoutFolder).parentId).toBeNull();
  });
});

describe('pickDocumentType (PR #918 uses mime_type + file_name)', () => {
  it.each([
    ['application/pdf', 'rules.pdf', 'pdf'],
    ['image/png', 'photo.png', 'image'],
    ['application/vnd.ms-excel', 'sheet.xlsx', 'spreadsheet'],
    ['text/plain', 'notes.txt', 'document'],
  ] as const)('classifies mime %s -> %s', (mime_type, file_name, expected) => {
    expect(pickDocumentType({ ...baseDoc, mime_type, file_name })).toBe(expected);
  });

  it('falls back to the file_name extension when mime_type is unhelpful', () => {
    expect(pickDocumentType({ ...baseDoc, mime_type: '', file_name: 'scan.PDF' })).toBe('pdf');
  });

  it('classifies spreadsheets by extension when mime is generic', () => {
    expect(
      pickDocumentType({ ...baseDoc, mime_type: 'application/octet-stream', file_name: 'data.csv' })
    ).toBe('spreadsheet');
  });
});
