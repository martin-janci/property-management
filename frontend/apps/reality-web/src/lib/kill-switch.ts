/**
 * Kill-switch HTML renderer (N8).
 *
 * Used by `src/middleware.ts` to short-circuit a request with an HTTP 503
 * when the per-tenant `building_disabled` feature flag is on. The route
 * NEVER renders normally in that branch — middleware returns this HTML
 * with `status: 503` and `Retry-After`, which is what CDN, monitoring, and
 * SEO crawlers actually want to see (a 200 is misread as "page works,
 * just shows a maintenance message").
 *
 * The HTML is intentionally a single self-contained string with inline
 * styles. Bootstrapping next-intl from middleware would require shipping
 * the i18n bundle to the edge runtime; the half-dozen labels we need are
 * cheap to inline by locale.
 *
 * Manual test:
 *   1. Toggle `building_disabled.enabled = true` for a test tenant.
 *   2. `curl -i https://acme.dev.example.com/` — expect:
 *        HTTP/1.1 503 Service Unavailable
 *        Content-Type: text/html; charset=utf-8
 *        Retry-After: 3600
 *   3. Toggle back off, expect 200.
 */

export type KillSwitchLocale = 'en' | 'sk' | 'cs' | 'de' | 'pl' | 'hu';

type Strings = {
  title: string;
  heading: string;
  body: string;
  retry: string;
};

const COPY: Record<KillSwitchLocale, Strings> = {
  en: {
    title: 'Service temporarily unavailable',
    heading: 'Service temporarily unavailable',
    body: 'This site is offline for maintenance. Please check back shortly.',
    retry: 'Please try again in a little while.',
  },
  sk: {
    title: 'Služba je dočasne nedostupná',
    heading: 'Služba je dočasne nedostupná',
    body: 'Stránka je odstavená kvôli údržbe. Skúste to prosím o chvíľu znovu.',
    retry: 'Skúste to prosím o chvíľu znovu.',
  },
  cs: {
    title: 'Služba je dočasně nedostupná',
    heading: 'Služba je dočasně nedostupná',
    body: 'Stránka je odstavena kvůli údržbě. Zkuste to prosím za chvíli znovu.',
    retry: 'Zkuste to prosím za chvíli znovu.',
  },
  de: {
    title: 'Dienst vorübergehend nicht verfügbar',
    heading: 'Dienst vorübergehend nicht verfügbar',
    body: 'Diese Website ist wegen Wartungsarbeiten offline. Bitte versuchen Sie es bald erneut.',
    retry: 'Bitte versuchen Sie es in Kürze erneut.',
  },
  pl: {
    title: 'Usługa tymczasowo niedostępna',
    heading: 'Usługa tymczasowo niedostępna',
    body: 'Ta strona jest niedostępna z powodu prac serwisowych. Spróbuj ponownie za chwilę.',
    retry: 'Spróbuj ponownie za chwilę.',
  },
  hu: {
    title: 'A szolgáltatás átmenetileg nem elérhető',
    heading: 'A szolgáltatás átmenetileg nem elérhető',
    body: 'Az oldal karbantartás miatt nem elérhető. Kérjük, próbálja újra rövidesen.',
    retry: 'Kérjük, próbálja újra rövidesen.',
  },
};

/** Coerce an arbitrary locale string to one we have copy for. */
export function normalizeKillSwitchLocale(input: string | null | undefined): KillSwitchLocale {
  if (!input) return 'en';
  const head = input.toLowerCase().split('-')[0];
  if (head === 'sk' || head === 'cs' || head === 'de' || head === 'pl' || head === 'hu') {
    return head;
  }
  return 'en';
}

/**
 * Render the maintenance HTML for a given locale.
 *
 * Output is a complete `<!doctype html>` document so it can be served as
 * the entire response body. Markup is intentionally noindex'd — Google
 * already drops 503s but it costs nothing to be explicit.
 */
export function renderKillSwitchHtml(localeInput: string | null | undefined): string {
  const locale = normalizeKillSwitchLocale(localeInput);
  const s = COPY[locale];
  // Inline styles only — middleware runs on the edge and we cannot rely on
  // the app's CSS being loaded.
  return `<!doctype html>
<html lang="${locale}">
<head>
<meta charset="utf-8" />
<meta name="viewport" content="width=device-width, initial-scale=1" />
<meta name="robots" content="noindex" />
<title>${escapeHtml(s.title)}</title>
<style>
  html,body{margin:0;padding:0;background:#fff;color:#111;font-family:system-ui,-apple-system,Segoe UI,Roboto,sans-serif}
  main{max-width:560px;margin:4rem auto;padding:0 1.25rem;text-align:center}
  h1{font-size:1.5rem;margin:0 0 .75rem}
  p{font-size:1rem;line-height:1.5;margin:0 0 .75rem;color:#444}
</style>
</head>
<body>
<main>
  <h1>${escapeHtml(s.heading)}</h1>
  <p>${escapeHtml(s.body)}</p>
  <p>${escapeHtml(s.retry)}</p>
</main>
</body>
</html>`;
}

function escapeHtml(input: string): string {
  return input
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;')
    .replace(/'/g, '&#39;');
}
