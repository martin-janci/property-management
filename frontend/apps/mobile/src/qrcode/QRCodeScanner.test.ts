/**
 * Code-review finding: the QR-code / deep-link parser functions exported from
 * `QRCodeScanner.ts` (`parseQRCode`, and `getQRCodeAction` which maps a parse
 * result to a navigation action) shipped with no unit coverage, unlike the
 * sibling `DeepLinkHandler` / `universalLinks` parsers. These are focused
 * characterization tests that pin the *current* behaviour across valid,
 * malformed, and edge-case inputs. They assert what the parser does today —
 * including a couple of known quirks noted inline — so future refactors stay
 * honest; they intentionally do NOT change parser behaviour.
 */
import { getQRCodeAction, parseQRCode } from './QRCodeScanner';
import type {
  BuildingAccessQRData,
  ContactQRData,
  DeepLinkQRData,
  DocumentQRData,
  ExternalURLQRData,
  FaultReportQRData,
  VoteQRData,
} from './types';

// The external-URL branch logs via `console.warn` under `__DEV__` (which
// jest-expo sets to true). Silence it so the suite output stays clean while
// still exercising that code path.
let warnSpy: jest.SpyInstance;
beforeEach(() => {
  warnSpy = jest.spyOn(console, 'warn').mockImplementation(() => undefined);
});
afterEach(() => {
  warnSpy.mockRestore();
});

describe('parseQRCode — PPT deep links (ppt://)', () => {
  it('treats a bare host as the screen with no params', () => {
    const result = parseQRCode('ppt://dashboard');
    expect(result.type).toBe('deep_link');
    expect(result.isValid).toBe(true);
    const data = result.data as DeepLinkQRData;
    expect(data.screen).toBe('dashboard');
    expect(data.params).toBeUndefined();
  });

  it('reads the screen + id from the pathname (triple-slash authority-less form)', () => {
    const result = parseQRCode('ppt:///faults/123');
    expect(result.isValid).toBe(true);
    const data = result.data as DeepLinkQRData;
    expect(data.screen).toBe('faults');
    expect(data.params).toEqual({ id: '123' });
  });

  it('collects query-string parameters into params', () => {
    const result = parseQRCode('ppt://documents?tab=shared&sort=date');
    expect(result.isValid).toBe(true);
    const data = result.data as DeepLinkQRData;
    // Quirk: with an authority present the host ("documents") is NOT used as the
    // screen — the empty pathname falls back to "dashboard". Pinned as-is.
    expect(data.screen).toBe('dashboard');
    expect(data.params).toEqual({ tab: 'shared', sort: 'date' });
  });

  it('exposes the first path segment as the screen (authority quirk)', () => {
    // Quirk: `new URL("ppt://faults/123")` parses "faults" as the host, so the
    // pathname is "/123" and the screen becomes "123". Characterized, not fixed.
    const result = parseQRCode('ppt://faults/123');
    const data = result.data as DeepLinkQRData;
    expect(data.screen).toBe('123');
  });
});

describe('parseQRCode — PPT JSON payloads', () => {
  it('parses a building_access payload and defaults accessLevel to "resident"', () => {
    const result = parseQRCode(
      '{"ppt":"1.0","type":"building_access","buildingId":"b1","accessPoint":"main-door"}'
    );
    expect(result.type).toBe('building_access');
    expect(result.isValid).toBe(true);
    const data = result.data as BuildingAccessQRData;
    expect(data.buildingId).toBe('b1');
    expect(data.accessPoint).toBe('main-door');
    expect(data.accessLevel).toBe('resident');
  });

  it('honours an explicit accessLevel', () => {
    const result = parseQRCode(
      '{"ppt":"1.0","type":"building_access","buildingId":"b1","accessPoint":"gate","accessLevel":"guest"}'
    );
    const data = result.data as BuildingAccessQRData;
    expect(data.accessLevel).toBe('guest');
  });

  it('parses a document payload', () => {
    const result = parseQRCode(
      '{"ppt":"1.0","type":"document","documentId":"d9","documentType":"invoice","buildingId":"b2"}'
    );
    expect(result.type).toBe('document');
    const data = result.data as DocumentQRData;
    expect(data).toMatchObject({ documentId: 'd9', documentType: 'invoice', buildingId: 'b2' });
  });

  it('parses a fault_report payload', () => {
    const result = parseQRCode(
      '{"ppt":"1.0","type":"fault_report","buildingId":"b3","category":"plumbing","location":"2F","equipmentId":"e7"}'
    );
    expect(result.type).toBe('fault_report');
    const data = result.data as FaultReportQRData;
    expect(data).toMatchObject({
      buildingId: 'b3',
      category: 'plumbing',
      location: '2F',
      equipmentId: 'e7',
    });
  });

  it('parses a vote payload', () => {
    const result = parseQRCode('{"ppt":"1.0","type":"vote","voteId":"v4","buildingId":"b5"}');
    expect(result.type).toBe('vote');
    const data = result.data as VoteQRData;
    expect(data).toMatchObject({ voteId: 'v4', buildingId: 'b5' });
  });

  it('rejects a JSON payload missing the "type" field as invalid', () => {
    const result = parseQRCode('{"ppt":"1.0"}');
    expect(result.type).toBe('unknown');
    expect(result.isValid).toBe(false);
    expect(result.error).toBe('Invalid JSON payload');
  });

  it('passes an unrecognised payload type through as the raw type with unknown data', () => {
    // Quirk: an unknown `type` yields unknown data but is still flagged valid.
    const result = parseQRCode('{"ppt":"1.0","type":"spaceship"}');
    expect(result.type).toBe('spaceship');
    expect(result.isValid).toBe(true);
    expect(result.data).toEqual({ type: 'unknown', content: '{"ppt":"1.0","type":"spaceship"}' });
  });

  it('marks malformed JSON (looks like a payload, does not parse) as invalid', () => {
    const result = parseQRCode('{"ppt": broken');
    expect(result.type).toBe('unknown');
    expect(result.isValid).toBe(false);
    expect(result.error).toBe('Invalid JSON payload');
  });
});

describe('parseQRCode — vCards', () => {
  it('parses a complete vCard with all supported fields', () => {
    const vcard = [
      'BEGIN:VCARD',
      'VERSION:3.0',
      'FN:Jane Doe',
      'TEL;TYPE=CELL:+421900123456',
      'EMAIL:jane@example.sk',
      'TITLE:Chairperson',
      'END:VCARD',
    ].join('\r\n');
    const result = parseQRCode(vcard);
    expect(result.type).toBe('contact');
    expect(result.isValid).toBe(true);
    const data = result.data as ContactQRData;
    expect(data).toMatchObject({
      name: 'Jane Doe',
      phone: '+421900123456',
      email: 'jane@example.sk',
      role: 'Chairperson',
    });
  });

  it('parses a plain TEL:/EMAIL: form (no TYPE parameter)', () => {
    const vcard = 'BEGIN:VCARD\nFN:Bob\nTEL:123456\nEMAIL:bob@e.sk\nEND:VCARD';
    const result = parseQRCode(vcard);
    const data = result.data as ContactQRData;
    expect(data.phone).toBe('123456');
    expect(data.email).toBe('bob@e.sk');
  });

  it('marks a vCard missing END:VCARD as invalid', () => {
    const result = parseQRCode('BEGIN:VCARD\nFN:Jane');
    expect(result.type).toBe('contact');
    expect(result.isValid).toBe(false);
    expect(result.error).toBe('Missing required vCard fields (BEGIN, END, or FN)');
  });

  it('marks a vCard missing FN (formatted name) as invalid', () => {
    const result = parseQRCode('BEGIN:VCARD\nTEL:123\nEND:VCARD');
    expect(result.isValid).toBe(false);
    const data = result.data as ContactQRData;
    expect(data.name).toBe('');
  });
});

describe('parseQRCode — external URLs', () => {
  it('accepts a well-formed https URL as an external_url', () => {
    const result = parseQRCode('https://example.com/path?x=1');
    expect(result.type).toBe('external_url');
    expect(result.isValid).toBe(true);
    const data = result.data as ExternalURLQRData;
    expect(data.url).toBe('https://example.com/path?x=1');
  });

  it('accepts a plain http URL', () => {
    const result = parseQRCode('http://example.org');
    expect(result.type).toBe('external_url');
    expect(result.isValid).toBe(true);
  });

  it('rejects a malformed URL that only looks like one', () => {
    const result = parseQRCode('https://');
    expect(result.type).toBe('unknown');
    expect(result.isValid).toBe(false);
    expect(result.error).toBe('Invalid URL format');
  });
});

describe('parseQRCode — unknown / edge cases', () => {
  it('returns unknown+invalid for arbitrary text', () => {
    const result = parseQRCode('just some random text');
    expect(result.type).toBe('unknown');
    expect(result.isValid).toBe(false);
    expect(result.error).toBe('Unrecognized QR code format');
  });

  it('returns unknown+invalid for an empty string', () => {
    const result = parseQRCode('');
    expect(result.type).toBe('unknown');
    expect(result.isValid).toBe(false);
  });

  it('trims surrounding whitespace before dispatching on the content', () => {
    const result = parseQRCode('   ppt://dashboard   ');
    expect(result.type).toBe('deep_link');
    expect(result.raw).toBe('ppt://dashboard');
    expect((result.data as DeepLinkQRData).screen).toBe('dashboard');
  });
});

describe('getQRCodeAction', () => {
  it('maps a deep_link to a navigate action carrying screen + params', () => {
    const action = getQRCodeAction(parseQRCode('ppt:///faults/123'));
    expect(action.action).toBe('navigate');
    expect(action.navigation).toBe('faults');
    expect(action.params).toEqual({ id: '123' });
  });

  it('maps a building_access payload to an access action', () => {
    const action = getQRCodeAction(
      parseQRCode(
        '{"ppt":"1.0","type":"building_access","buildingId":"b1","accessPoint":"main-door"}'
      )
    );
    expect(action.action).toBe('access');
    expect(action.confirmation).toContain('main-door');
  });

  it('maps a document payload to a Documents navigation', () => {
    const action = getQRCodeAction(
      parseQRCode('{"ppt":"1.0","type":"document","documentId":"d9","documentType":"invoice"}')
    );
    expect(action.action).toBe('navigate');
    expect(action.navigation).toBe('Documents');
    expect(action.params).toEqual({ documentId: 'd9' });
  });

  it('maps a fault_report payload to a ReportFault navigation, coalescing optional fields', () => {
    const action = getQRCodeAction(
      parseQRCode('{"ppt":"1.0","type":"fault_report","buildingId":"b3"}')
    );
    expect(action.navigation).toBe('ReportFault');
    expect(action.params).toEqual({ buildingId: 'b3', category: '', location: '' });
  });

  it('maps a vote payload to a Voting navigation', () => {
    const action = getQRCodeAction(
      parseQRCode('{"ppt":"1.0","type":"vote","voteId":"v4","buildingId":"b5"}')
    );
    expect(action.navigation).toBe('Voting');
    expect(action.params).toEqual({ voteId: 'v4' });
  });

  it('maps a contact to a save_contact action', () => {
    const action = getQRCodeAction(parseQRCode('BEGIN:VCARD\nFN:Jane\nEND:VCARD'));
    expect(action.action).toBe('save_contact');
    expect(action.confirmation).toContain('Jane');
  });

  it('maps an external_url to an open_url action', () => {
    const action = getQRCodeAction(parseQRCode('https://example.com'));
    expect(action.action).toBe('open_url');
    expect(action.confirmation).toContain('https://example.com');
  });

  it('maps an unknown parse result to an unknown action', () => {
    const action = getQRCodeAction(parseQRCode('garbage'));
    expect(action.action).toBe('unknown');
    expect(action.confirmation).toBe('Unknown QR code');
  });
});
