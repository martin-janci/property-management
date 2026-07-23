---
id: ppt/login
name: Login
product: ppt
sitemapRefs:
  ppt-web: ppt-login
implementations:
  ppt-web:
    component: LoginPage
    buildStatus: shipped
    redesignStatus: in-progress
    apiStatus: complete
  mobile:
    buildStatus: n/a
    redesignStatus: n/a
    apiStatus: n/a
endpoints:
  - auth_login
relatedScreens:
  - id: ppt/home
    rel: child
  - id: ppt/auth-callback
    rel: child
sharedComponents:
  - text-input
  - validation-patterns
  - banner
  - wizard
  - segmented-control
  - sso-buttons
designSources:
  - adapter: claude-design
    file: guest-registration-v2-design-system/project/pages/ppt-login.html
    frame: row1-loaded+magic / row2-totp+sms / row3-loading+empty / row4-errors-3up
useCases:
  - UC-14
epics:
  - Epic-1
diagrams: []
owner: pm-frontend
---

## Functionality Checklist

<!-- tag with [w] / [m] / [w,m] / [-] -->

### Brand + chrome
- [ ] [w] Centered card on `bg-art` decorative background; max-width ~440px
- [ ] [w] Brand block: P-mark + "Property Tracker" + "Správa nehnuteľností" tagline
- [ ] [w] H1 "Vitajte späť" + sub "Prihláste sa, aby ste videli svoje nehnuteľnosti, hlásenia a hlasovania."

### Method tabs (Heslo / Magic link)
- [ ] [w] 2-tab segmented control above the field stack (`mtabs`); single-select; tab swap reflows the form below without leaving the screen

### Email + password (default)
- [ ] [w] Email field with `mail` leading icon; type=email; default value pre-populated when memorized
- [ ] [w] Password field with `lock` leading icon + `eye` trailing toggle (show/hide); `is-error` state ring on validation fail
- [ ] [w] Inline `field-error` row below field with circle-info icon when validation fails
- [ ] [w] Row: "Zostať prihlásený" checkbox left + "Zabudli ste heslo?" link right
- [ ] [w] Primary submit button `Prihlásiť sa` (full-width)

### Magic link variant
- [ ] [w] Hides password + remember + submit; replaces with `sent-card`: envelope icon + "Skontrolujte si e-mail" + body referencing the entered email + "Odkaz je platný 15 minút"
- [ ] [w] Info banner (`banner.info`): "Otvorte odkaz na zariadení, kde sa chcete prihlásiť."
- [ ] [w] Resend countdown line: "Neprišiel? Pošleme znova o 0:42 · Zmeniť e-mail"

### SSO row + divider
- [ ] [w] Divider with center label "alebo"
- [ ] [w] 2-column SSO buttons: Google (multicolor brand SVG) + Apple (mono); third full-width button "Pracovný účet (SAML/OIDC)" with grid icon

### Footer link
- [ ] [w] "Nemáte účet? Kontaktujte správcu" muted

### 2FA — TOTP step
- [ ] [w] H1 "Overovací kód" + sub mentioning 1Password / Google Authenticator / Authy
- [ ] [w] OTP field: 6 single-digit inputs (`otp.d`), filled cells get `.has` styling (brand-tinted), autofocus on next empty cell
- [ ] [w] Label hint "Kód · obnoví sa o 18 s" — countdown visible
- [ ] [w] Primary CTA "Overiť"; muted alt link "Nemám prístup ku kódu — použiť záložný kód"

### 2FA — SMS step
- [ ] [w] H1 "Kód z SMS" + sub showing masked phone "+421 ••• ••• 412"
- [ ] [w] Same OTP grid pattern as TOTP
- [ ] [w] Resend countdown "Poslať znova o 0:25" + alt "Použiť autentifikátor"

### Loading (submit in-flight)
- [ ] [w] Email + password fields disabled; submit button shows spinner + "Prihlasujeme…"; SSO + tabs hidden during submit

### Empty (fresh tab)
- [ ] [w] Pure placeholders ("vase@meno.sk", "••••••••••"); no remember-me prefilled; SSO row visible

### Error · wrong credentials
- [ ] [w] Top `banner.err` with circle-info icon: bold "Nesprávny e-mail alebo heslo." + body "Skontrolujte preklepy. Zostávajú vám 3 pokusy."
- [ ] [w] Password field gets `is-error` ring + inline field error "Skontrolujte heslo"
- [ ] [w] Email + remember + forgot stay enabled

### Error · locked-out
- [ ] [w] Replaces form with `lockcard`: large lock icon + "Účet je dočasne uzamknutý" + "Po 5 neúspešných pokusoch sme prihlasovanie pozastavili. Skúste znova o:" + 14:32 countdown (large, tabular-nums)
- [ ] [w] Alt actions: "resetujte heslo cez e-mail" link · "kontaktujte správcu" link

### Error · network
- [ ] [w] `banner.warn` (wifi-off icon) above form: bold "Server momentálne neodpovedá." + body "Skontrolujte pripojenie a skúste to znova. <a>Stav služby</a>"
- [ ] [w] Submit becomes "Skúsiť znova" with refresh icon; fields keep entered values

### Locale + theme switcher (artboard chrome only)
- [ ] [-] preview-bar with Theme (Light/Dark) + Locale (SK/CS/DE/EN)

## States

- **Empty (fresh tab)**: no values, placeholders only, both tabs visible, SSO row visible. No banners.
- **Loading (submit in flight)**: fields disabled, submit shows spinner with "Prihlasujeme…", magic-link/SSO tabs hidden until response.
- **Error**: 3 explicit variants depicted (wrong creds inline + field, locked-out lock-card replacing form with countdown, network outage with warn banner + retry CTA).
- **Success / loaded (default)**: pre-filled email, password masked, "Zostať prihlásený" checked, all SSO options visible, primary CTA enabled.
- **2FA TOTP / SMS** (post-password): separate step replaces the password card; OTP grid + countdown; alt-method links between TOTP and SMS.
- **Magic-link sent**: confirmation card with envelope icon + 15-min validity + resend countdown; replaces password fields entirely.

## Notes

### Broader context

UC-14 single sign-in entry point. Email + password is primary; magic-link, TOTP, SMS, and SSO (Google / Apple / SAML/OIDC) are alternatives. After the first factor, the design routes to TOTP or SMS based on account settings (UC-23 second-factor preference). Locked-out state activates after 5 failed attempts (server-enforced, 15-minute cooldown shown in countdown).

### Specific (recent)

- The brand mark is a "P" tile + "Property Tracker · Správa nehnuteľností" wordmark — the bundle's invented PPT brand (per project README caveat 2). Production should swap when the real brand asset lands.
- `mtabs` segmented (Heslo / Magic link) lives above the email field — the tab swap should preserve the entered email value (don't blank).
- Email field uses `lead` icon SVG inline; password adds a `trail` button (eye icon) for show/hide. The trailing button must have `aria-label="Zobraziť heslo"` (toggling to "Skryť heslo" when on).
- OTP grid: 6 single-digit inputs, `maxlength=1`, advance focus on input, backspace shifts focus left. `.has` modifier is brand-tinted background indicating filled. The 18s refresh hint must update via `setInterval` countdown, not a static label.
- Locked-out countdown is `MM:SS` tabular-nums; uses real server-side TTL, not a client clock.
- 4 locales bundled (SK / CS / DE / EN) with full string maps in the artboard's inline JS — implementation should pull these into `messages/sk.json` etc. SK is canonical (artboard's `data-i18n` text is SK).
- Magic-link copy: "platný 15 minút" — must come from server config (not hardcoded). Resend cadence "0:42" implies 60s minimum throttle.
- SSO buttons: Google brand SVG includes the official 4-color path data; Apple is mono using `currentColor`; SAML/OIDC uses a generic 4-square grid icon. When porting, do not re-stylize Google/Apple marks — brand guidelines apply.
- Banner variants: `info` (blue circle-info), `warn` (amber wifi-off), `err` (red circle-info). Tokens come from the 8-state-machine system; banners should reuse `forms/banner.html` patterns.
- Field-error row uses `<svg>` (circle-info) + label tied to the field's `aria-describedby`. Don't show the inline error AND a top banner for the same issue — pick one (banner for system, inline for field-specific).
- Service-status link in network-error banner ("<a>Stav služby</a>") should point to a public statuspage URL — TBD at deploy.
- DE strings: "Anmeldung läuft…" is longer than "Prihlasujeme…" — submit button must wrap or use icon-only spinner on narrow widths.
- Mobile (RN) login is `n/a` per current screen-map — flag if a mobile login UI gets requested; this design is web-only.
- Post-login return-URL handling is now open-redirect hardened (PR #922, dev-review round 2). The `getAndClearReturnUrl()` / `setReturnUrl()` helpers in `@ppt/shared` both run `sanitizeReturnUrl()` — same-origin rooted paths only; absolute (`https://evil.com`), protocol-relative (`//host`, `/\host`), scheme (`javascript:`), and control-char URLs are rejected on BOTH write and read (defense in depth catches tampered/old stored values). The login redirect target fed into `navigate()` can no longer bounce a freshly-authenticated user off-site.

## Agent Log

<!-- newest entries on top -->

- 2026-06-03 — agent: test-gap-screen-map-drift-pr-922-ppt — reconciled drift from PR #922 (dev-review round 2): `@ppt/shared` `setReturnUrl`/`getAndClearReturnUrl` now sanitize via new `sanitizeReturnUrl` (same-origin rooted paths only; rejects absolute/protocol-relative/scheme/control-char URLs on write AND read) — closes post-login open-redirect. No frontmatter change (shipped/complete).
- 2026-05-27 — agent: gap-79-2-login-flow-wiring — wired TanStack Query cache clear (queryClient.clear()) into logout() for session cleanup; loginWithSsoCode already present; apiStatus promoted to complete
- 2026-05-24 — agent: gap-79-2 login-flow-wiring — removed local RETURN_URL_KEY constant and inline getAndClearReturnUrl; now imports getAndClearReturnUrl from @ppt/shared for consistent return-URL handling across auth flows
- 2026-05-09 — agent: design analyzed (pages/ppt-login.html — 4 rows: loaded+magic / TOTP+SMS / loading+empty / 3 errors); flipped ppt-web redesignStatus → in-progress; attached designSource; populated functionality checklist (12 sections covering 6 distinct UI states), all states with full variants, design-specific notes; declared 6 sharedComponents; added 1 relatedScreen
- 2026-05-08 — init: created from scan (source: sitemap)
