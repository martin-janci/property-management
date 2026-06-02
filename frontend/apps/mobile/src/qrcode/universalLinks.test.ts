/**
 * gap-85-3: regression tests for universal-link (iOS) / App-Link (Android)
 * URL normalisation and end-to-end deep-link parsing.
 */
import { parseDeepLink } from './DeepLinkHandler';
import { DEFAULT_APP_LINK_HOST, normalizeLinkUrl } from './universalLinks';

describe('normalizeLinkUrl', () => {
  it('normalises a custom-scheme link with path + query', () => {
    expect(normalizeLinkUrl('ppt://faults/123?source=qr')).toBe('faults/123?source=qr');
  });

  it('normalises the ppt-management scheme', () => {
    expect(normalizeLinkUrl('ppt-management://documents/9')).toBe('documents/9');
  });

  it('normalises an HTTPS universal link on the associated host', () => {
    expect(normalizeLinkUrl(`https://${DEFAULT_APP_LINK_HOST}/faults/123?source=email`)).toBe(
      'faults/123?source=email'
    );
  });

  it('rejects an HTTPS link on a foreign host', () => {
    expect(normalizeLinkUrl('https://evil.example.org/faults/123')).toBeNull();
  });

  it('rejects an unsupported scheme', () => {
    expect(normalizeLinkUrl('mailto:someone@example.com')).toBeNull();
  });

  it('rejects a malformed URL', () => {
    expect(normalizeLinkUrl('not a url')).toBeNull();
  });
});

describe('parseDeepLink — universal links', () => {
  it('parses an HTTPS universal link to the matching screen + params', () => {
    const result = parseDeepLink(`https://${DEFAULT_APP_LINK_HOST}/faults/42`);
    expect(result.success).toBe(true);
    expect(result.screen).toBe('Faults');
    expect(result.params).toEqual({ id: '42' });
    expect(result.requiresAuth).toBe(true);
  });

  it('parses query parameters from a universal link', () => {
    const result = parseDeepLink(`https://${DEFAULT_APP_LINK_HOST}/documents?tab=shared`);
    expect(result.success).toBe(true);
    expect(result.screen).toBe('Documents');
    expect(result.query).toEqual({ tab: 'shared' });
  });

  it('still parses the legacy ppt:// custom scheme', () => {
    const result = parseDeepLink('ppt://announcements/7');
    expect(result.success).toBe(true);
    expect(result.screen).toBe('Announcements');
    expect(result.params).toEqual({ id: '7' });
  });

  it('fails for an HTTPS link on a non-associated host', () => {
    const result = parseDeepLink('https://phishing.example.net/faults/42');
    expect(result.success).toBe(false);
  });

  it('fails for an unknown route on the associated host', () => {
    const result = parseDeepLink(`https://${DEFAULT_APP_LINK_HOST}/unknown/path`);
    expect(result.success).toBe(false);
  });
});
