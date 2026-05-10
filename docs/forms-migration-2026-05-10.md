# Reality Portal — form system migration plan

Source design: `guest-registration-v2-design-system/project/pages/sell.html`
(plus `profile.html`, `contact.html`, `listing-edit-step-1..5.html`).

The design ships a **modern form system** based on floating-label inputs,
custom dropdowns, and consistent focus rings. This doc captures the
specification and the migration plan.

## Primitives now available

Importable from `@/components/forms`:

| Component          | Notes                                                       |
|--------------------|-------------------------------------------------------------|
| `FieldInput`       | Floating label, prefix/suffix slots, error + helper text.   |
| `FieldTextarea`    | Same UX, multi-line, resizable.                             |
| `FieldSelect`      | Custom popover dropdown — keyboard nav, search-ready slot.  |
| `FormCard`         | White-surface section card with title + optional step pill. |
| `FieldGroupTitle`  | Uppercase mini-heading with horizontal divider line.        |
| `FormActions`      | Sticky bottom action bar.                                   |

All read existing `--ppt-*` tokens, so dark mode pairs automatically and
no token migration is needed in consumers.

## Spec summary

Distilled from `sell.html` `<style>` blocks:

- **Field padding**: `22px 14px 8px` (top wider than bottom — label parks there at rest, animates up on focus/value).
- **Radius**: `10px` on inputs, `12px` on form-card.
- **Focus**: `border-color: var(--ppt-color-primary)` + `box-shadow: 0 0 0 4px rgba(37,99,235,.12)` (4-px ring at 12% opacity — softer than the standard focus ring used elsewhere).
- **Hover (idle)**: `border-color: var(--ppt-neutral-400)`.
- **Label animation**: idle = `13.5px / 500-weight / muted` at top:14px; floated = `11px / 600-weight / uppercase / letter-spacing:.04em` at translateY(-9px).
- **Error**: border + ring switch to `--ppt-color-danger` + 12% red ring.
- **Dropdown menu**: `var(--ppt-bg-elevated)` background, 12px radius, popover shadow, max-height 320 with internal scroll.
- **Step pill** (inside section h2): `11px / 700`, `--ppt-color-primary-soft-bg` background, accent-tinted text, pill radius 999.
- **Field group title**: `11px / 700 / uppercase / letterspacing:.06em` with a hairline `::after { flex:1; height:1px }` divider.

## Migration plan — per route

| Route               | Form size           | Components used                                                            | Status        |
|---------------------|---------------------|----------------------------------------------------------------------------|---------------|
| `/auth/login`       | 2 fields            | `FieldInput`                                                               | **DONE** 2026-05-10 |
| `/auth/register`    | 4 fields            | `FieldInput`                                                               | TODO          |
| `/contact`          | 4 fields + textarea | `FieldInput`, `FieldTextarea`, `FormCard`                                  | TODO          |
| `/report`           | 5 fields + textarea | `FieldInput`, `FieldTextarea`, `FieldSelect`, `FormCard`                   | TODO          |
| `/profile`          | 6 fields            | `FieldInput`, `FieldSelect`, `FormCard`, `FieldGroupTitle`                 | TODO          |
| `/account/password` | 3 fields            | `FieldInput`, `FormCard`                                                   | TODO          |
| `/sell` (single)    | 14 fields           | `FieldInput`, `FieldTextarea`, `FieldSelect`, `FormCard`, `FieldGroupTitle`, `FormActions` | TODO — design has a 5-step wizard instead of one page; consider rebuilding as wizard during migration |
| `/saved-searches`   | Modal form          | All of the above                                                           | TODO          |
| `/inquiries` (reply)| Textarea + actions  | `FieldTextarea`, `FormActions`                                             | TODO          |
| `/agency/branding`  | Logo + colors + …   | `FieldInput`, `FormCard`                                                   | TODO          |
| `/agency/import`    | File drop + mapping | `FieldSelect`, `FormCard`, `FormActions`                                   | TODO (largest) |

## Migration order suggestion

1. **`/auth/register`** — same shape as login, fastest second migration.
2. **`/contact`** — small, standalone, low-risk visual.
3. **`/report`** — single-card form with a select; tests `FieldSelect` for the first time in a real form.
4. **`/account/password`** — small form inside the account shell.
5. **`/profile`** — multi-section profile with `FieldGroupTitle`.
6. **`/sell` rebuild as 5-step wizard** — biggest visual change; follow the design's `listing-edit-step-{1..5}.html` flow with shared header + step indicator.
7. **`/saved-searches` modal** — reuses sell-style filters in a modal.
8. **Agency suite** — last because the surfaces are deeper.

## Caveats

1. The `:not(:placeholder-shown)` trick the floating-label animation depends on requires every field input to be rendered with `placeholder=" "` (a single space). `Field.tsx` does this automatically — callers MUST NOT pass a `placeholder` prop. Type signature already strips it.
2. `FieldSelect` is a custom popover (not native `<select>`). Consumers wanting form-data participation get a hidden `<input name=…>` for free; for `react-hook-form` register-style usage, prefer the controlled `value` / `onChange` API.
3. Forms `.css` overrides global `.field` class. If any existing component used `.field` as a class (login form did, now migrated), check for visual regressions during migration.
