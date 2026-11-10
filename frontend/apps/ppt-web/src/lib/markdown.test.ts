/// <reference types="vitest/globals" />
import { describe, expect, it } from 'vitest';
import { markdownToHtml, renderMarkdownSafe } from './markdown';

describe('markdownToHtml', () => {
  it('renders headings, emphasis, lists and paragraphs', () => {
    const html = markdownToHtml(
      '## Oznámenie\n\nUpozorňujeme na **prerušenie** vody.\n\n- Naplniť nádobu\n- Neprať\n\n### Program\n1. Správa\n2. Rozpočet'
    );
    expect(html).toContain('<h2>Oznámenie</h2>');
    expect(html).toContain('<p>Upozorňujeme na <strong>prerušenie</strong> vody.</p>');
    expect(html).toContain('<ul><li>Naplniť nádobu</li><li>Neprať</li></ul>');
    expect(html).toContain('<h3>Program</h3>');
    expect(html).toContain('<ol><li>Správa</li><li>Rozpočet</li></ol>');
  });

  it('honours hard line breaks (two trailing spaces)', () => {
    expect(markdownToHtml('**Dátum:** Utorok  \n**Čas:** 18:00')).toContain(
      '<strong>Dátum:</strong> Utorok<br>\n<strong>Čas:</strong> 18:00'
    );
  });

  it('renders italics, inline code and safe links', () => {
    const html = markdownToHtml('*pozor* `kód` [web](https://example.com)');
    expect(html).toContain('<em>pozor</em>');
    expect(html).toContain('<code>kód</code>');
    expect(html).toContain(
      '<a href="https://example.com" target="_blank" rel="noopener noreferrer">web</a>'
    );
  });
});

describe('renderMarkdownSafe', () => {
  it('escapes raw HTML instead of rendering it', () => {
    const html = renderMarkdownSafe('<img src=x onerror="alert(1)"><script>alert(1)</script>');
    expect(html).not.toContain('<img');
    expect(html).not.toContain('<script');
    expect(html).toContain('&lt;script&gt;');
  });

  it('drops javascript: links', () => {
    const html = renderMarkdownSafe('[klik](javascript:alert(1))');
    expect(html).not.toContain('href="javascript');
    expect(html).not.toContain('<a');
  });

  it('returns an empty string for empty input', () => {
    expect(renderMarkdownSafe('')).toBe('');
    expect(renderMarkdownSafe(undefined)).toBe('');
  });
});
