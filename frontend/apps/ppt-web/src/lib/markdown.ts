/**
 * Minimal, dependency-free Markdown → HTML renderer for user-authored content
 * (announcement bodies). Supports the subset managers actually write:
 * headings (#–######), paragraphs, hard line breaks (two trailing spaces),
 * unordered (-, *, +) and ordered (1.) lists, blockquotes, horizontal rules,
 * **bold**, *italic* / _italic_, `inline code` and [links](https://…).
 *
 * Safety: the source is HTML-escaped before any markup is produced, links are
 * restricted to http(s)/mailto/tel and relative paths, and the result is run
 * through DOMPurify with a strict allowlist as defense-in-depth.
 */
import DOMPurify from 'dompurify';

const ALLOWED_TAGS = [
  'h1',
  'h2',
  'h3',
  'h4',
  'h5',
  'h6',
  'p',
  'br',
  'ul',
  'ol',
  'li',
  'strong',
  'em',
  'code',
  'a',
  'blockquote',
  'hr',
];

function escapeHtml(text: string): string {
  return text
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;')
    .replace(/'/g, '&#39;');
}

function safeHref(rawHref: string): string | null {
  // rawHref is already HTML-escaped; decode &amp; for the scheme check only.
  const href = rawHref.trim().replace(/&amp;/g, '&');
  if (/^(https?:|mailto:|tel:)/i.test(href) || /^\/(?!\/)/.test(href) || href.startsWith('#')) {
    return rawHref.trim();
  }
  return null;
}

/** Render inline markup on an already-escaped line. */
function renderInline(escaped: string): string {
  // Protect inline code spans from further processing.
  const codeSpans: string[] = [];
  let out = escaped.replace(/`([^`]+)`/g, (_m, code: string) => {
    codeSpans.push(`<code>${code}</code>`);
    return `\u0000${codeSpans.length - 1}\u0000`;
  });

  out = out.replace(/\[([^\]]+)\]\(([^)\s]+)\)/g, (match, label: string, href: string) => {
    const safe = safeHref(href);
    return safe
      ? `<a href="${safe}" target="_blank" rel="noopener noreferrer">${label}</a>`
      : match;
  });
  out = out.replace(/\*\*([^*]+)\*\*/g, '<strong>$1</strong>');
  out = out.replace(/__([^_]+)__/g, '<strong>$1</strong>');
  out = out.replace(/(^|[^*])\*([^*\s][^*]*)\*/g, '$1<em>$2</em>');
  out = out.replace(/(^|[^\w_])_([^_\s][^_]*)_(?!\w)/g, '$1<em>$2</em>');

  // biome-ignore lint/suspicious/noControlCharactersInRegex: NUL is the placeholder delimiter
  return out.replace(/\u0000(\d+)\u0000/g, (_m, i: string) => codeSpans[Number(i)] ?? '');
}

/** Join paragraph lines, honouring Markdown hard breaks (two trailing spaces). */
function joinLines(lines: string[]): string {
  return lines
    .map((line, i) => {
      const hardBreak = / {2,}$/.test(line) && i < lines.length - 1;
      return renderInline(line.trim()) + (hardBreak ? '<br>' : '');
    })
    .join('\n');
}

export function markdownToHtml(markdown: string): string {
  const lines = escapeHtml(markdown.replace(/\r\n?/g, '\n')).split('\n');
  const html: string[] = [];
  let paragraph: string[] = [];
  // `as` keeps TS from narrowing to `null` (the flush closures reassign it).
  let list = null as { type: 'ul' | 'ol'; items: string[] } | null;
  let quote: string[] = [];

  const flushParagraph = () => {
    if (paragraph.length) html.push(`<p>${joinLines(paragraph)}</p>`);
    paragraph = [];
  };
  const flushList = () => {
    if (list) {
      html.push(
        `<${list.type}>${list.items.map((item) => `<li>${renderInline(item)}</li>`).join('')}</${list.type}>`
      );
    }
    list = null;
  };
  const flushQuote = () => {
    if (quote.length) html.push(`<blockquote><p>${joinLines(quote)}</p></blockquote>`);
    quote = [];
  };
  const flushAll = () => {
    flushParagraph();
    flushList();
    flushQuote();
  };

  for (const line of lines) {
    const trimmed = line.trim();
    if (!trimmed) {
      flushAll();
      continue;
    }

    const heading = /^(#{1,6})\s+(.*?)\s*#*$/.exec(trimmed);
    if (heading) {
      flushAll();
      const level = heading[1].length;
      html.push(`<h${level}>${renderInline(heading[2])}</h${level}>`);
      continue;
    }

    if (/^([-*_])(\s*\1){2,}$/.test(trimmed)) {
      flushAll();
      html.push('<hr>');
      continue;
    }

    const quoteMatch = /^&gt;\s?(.*)$/.exec(trimmed);
    if (quoteMatch) {
      flushParagraph();
      flushList();
      quote.push(quoteMatch[1]);
      continue;
    }

    const ul = /^[-*+]\s+(.*)$/.exec(trimmed);
    const ol = /^\d+[.)]\s+(.*)$/.exec(trimmed);
    if (ul || ol) {
      flushParagraph();
      flushQuote();
      const type = ul ? 'ul' : 'ol';
      if (!list || list.type !== type) {
        flushList();
        list = { type, items: [] };
      }
      list.items.push((ul ?? ol)?.[1] ?? '');
      continue;
    }

    flushList();
    flushQuote();
    paragraph.push(line);
  }
  flushAll();

  return html.join('\n');
}

/** Markdown → sanitized HTML, safe for `dangerouslySetInnerHTML`. */
export function renderMarkdownSafe(markdown: string | null | undefined): string {
  if (!markdown) return '';
  return DOMPurify.sanitize(markdownToHtml(markdown), {
    ALLOWED_TAGS,
    ALLOWED_ATTR: ['href', 'target', 'rel'],
    ALLOW_DATA_ATTR: false,
  });
}
