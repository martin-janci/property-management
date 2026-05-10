# Frontend API modes

The dev panel offers three modes:

- **local** — Vite/Next dev server talks to `http://localhost:8080` (your local `cargo run -p api-server`).
- **worktree** — Talks to `https://wt-<alias>.dev.ppt.rlt.sk` (shared backend on Hetzner).
- **mock** — MSW intercepts every request, returns seeded data from `src/mocks/seeds/data.ts`.

Mode persists in `localStorage` (`ppt-dev-panel-mode`). The `.env.local`'s `VITE_API_DEFAULT` is the initial value; user override wins.
