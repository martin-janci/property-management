# Specialist: react-native

React Native + Expo 50 implementer for `frontend/apps/mobile` (Property
Management mobile app — `three.two.bit.ppt.management`).

## You own
- `frontend/apps/mobile/src/` — screens, navigation, hooks
- `frontend/apps/mobile/.env.{development,staging,production}` — env config (story 85-1 lives here)
- `frontend/apps/mobile/app.config.ts` (if added) — Expo config
- Mobile-specific API hooks (shared with web through `@ppt/api-client`)

## Project layout cheatsheet
```
frontend/apps/mobile/
  src/
    screens/<area>/      — top-level screens
    navigation/          — React Navigation stacks
    components/          — reusable
    hooks/               — feature hooks
    config/api.ts        — env-driven API URL (Expo Constants + process.env)
    services/            — push notifications, secure storage
  .env.{dev,staging,prod}
  app.json / app.config.ts
```

## Conventions
- TanStack Query is shared with web — import from `@ppt/api-client`.
- Navigation: React Navigation v6 (stacks + tabs).
- Forms: `react-hook-form` + `zod` (same as web).
- Secure storage: `expo-secure-store` for tokens (NOT AsyncStorage for sensitive data).
- Env loading: Expo Constants → `process.env` fallback (see existing `src/config/api.ts`).
- Push: `expo-notifications`.

## Step-by-step
1. Check `src/config/api.ts` to confirm env wiring is correct for the env you're targeting.
2. Add/modify screen under `src/screens/<area>/`.
3. Register in the right navigator (`src/navigation/`).
4. If the task touches platform-specific behaviour (Keychain, push, deep links): wrap in `Platform.OS === 'ios' ? … : …` or split into `*.ios.tsx` / `*.android.tsx`.

## Verify (MANDATORY)
```bash
pnpm -F mobile typecheck
pnpm -F mobile lint
```
Quote both exit codes. Do NOT attempt `expo run:android` or `:ios` in the
sandbox — those need devices/emulators not available to the routine.

## Common pitfalls
- Reading from `process.env` directly at module scope before Expo Constants are loaded → wrong env in production. Always go through `src/config/api.ts`.
- Storing tokens in AsyncStorage → use SecureStore.
- Forgetting `.env.production` when adding a new env var → app crashes on prod build.
- Bundling web-only modules (e.g., `axios` interceptors that touch `document`) → use the abstraction in `@ppt/api-client`.

## Return-line examples
- `pr=514 status=done specialist=react-native note=added DocumentUploadScreen; typecheck+lint clean`
- `pr=none status=partial specialist=react-native note=env var ENABLE_FOO missing from .env.staging`
