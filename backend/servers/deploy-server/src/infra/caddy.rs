// backend/servers/deploy-server/src/infra/caddy.rs
use crate::Result;
use serde_json::json;
use tracing::{error, info, instrument, warn};

// Loop bound and total deadline for `unregister_route`'s DELETE sweep.
// Kept as `const` so the bound and the diagnostic strings can never drift.
const MAX_DELETE_ITERS: usize = 16;
const SWEEP_DEADLINE: std::time::Duration = std::time::Duration::from_secs(15);

/// UTF-8-safe byte truncation. `&s[..n]` panics if `n` falls inside a
/// multi-byte codepoint, which would crash the error-logging path the
/// first time Caddy ever returned a non-ASCII body at the wrong offset.
/// Walks back to the nearest char boundary on or below `max_bytes`.
fn truncate_utf8(s: &str, max_bytes: usize) -> &str {
    if s.len() <= max_bytes {
        return s;
    }
    let mut end = max_bytes;
    while end > 0 && !s.is_char_boundary(end) {
        end -= 1;
    }
    &s[..end]
}

pub struct CaddyClient {
    base: String,
    http: reqwest::Client,
}

impl CaddyClient {
    pub fn new(base: impl Into<String>) -> Self {
        // Bounded timeouts on every Caddy admin call. These are reached
        // synchronously from request handlers (open/close worktree, GC tick,
        // promote/rollback) and the calling task usually holds a per-worktree
        // lock — a wedged Caddy admin API would otherwise stall the whole
        // operation indefinitely and starve other open/close calls for the
        // same worktree.
        //
        // Connect timeout is short because `localhost:2019` is a unix-socket
        // proxy or in-host TCP — a slow connect means Caddy is dead. Total
        // timeout includes payload upload (route JSON is small) so a 5s budget
        // is generous.
        let http = reqwest::Client::builder()
            .connect_timeout(std::time::Duration::from_secs(2))
            .timeout(std::time::Duration::from_secs(5))
            .build()
            .expect("build Caddy HTTP client");
        Self {
            base: base.into(),
            http,
        }
    }

    /// Register a host → upstream mapping. Idempotent: always leaves Caddy
    /// with exactly one ppt-deploy-managed route (carrying `@id`
    /// `ppt-deploy-<sanitized-host>`) for `host`, regardless of how many
    /// times it's called or what the prior `@id` index state was. Routes
    /// for the same host added by other writers (manual `curl`, foreign
    /// `@id`) are untouched.
    ///
    /// Strategy: DELETE-by-id (looped — see `unregister_route`) first, then
    /// POST-append a fresh route to `apps.http.servers.srv0.routes`.
    ///
    /// **Retry-safe on transport errors**: if a transient `reqwest` failure
    /// surfaces mid-call, the caller can simply re-invoke `register_route`
    /// with the same args — the DELETE sweep at the top will clean up any
    /// route the lost POST may have committed.
    ///
    /// **On POST failure**: the DELETE sweep has already cleared any prior
    /// route for `host`, so a failed POST leaves the host with ZERO routes
    /// (502s until the next successful call). The failure is logged at
    /// `error!` level with full context AND returned as `Err`.
    ///
    /// The earlier PUT-by-id-with-POST-fallback was supposed to be idempotent
    /// but observed behaviour on long-running Caddy instances showed duplicate
    /// routes accumulating across redeploys: PUT returned 404 every time
    /// (Caddy's `@id` annotation is per-config-write, and routes appended via
    /// `/routes/...` without explicit `@id` in older deploy-server versions
    /// weren't tagged), so the fallback POST appended a new entry on each
    /// call. After three deploys, three routes for `rlt.sk` — each pointing
    /// at a different blue/green color, and Caddy dispatches to the FIRST
    /// match, frequently the dead container. Forcing DELETE first guarantees
    /// the array contains exactly one managed entry per host after this call.
    ///
    /// Requires Caddy >= 2.0 (`@id` lookup on `/id/<id>` endpoints).
    ///
    /// **Single-instance assumption (CHAOS-001)**: deploy-server is assumed
    /// to run as a single process per target. Two instances racing this
    /// method against the same host serialize at Caddy admin, but both
    /// POSTs append — leaving 2 routes for the host until the next call
    /// from either instance sweeps. No coordination here; if you ever run
    /// deploy-server in HA, add a distributed lock or a post-POST GET-by-id
    /// verify (expect count=1).
    #[instrument(skip(self), fields(host = %host, upstream = %upstream))]
    pub async fn register_route(&self, host: &str, upstream: &str) -> Result<()> {
        // DELETE-loop sweeps any stale routes carrying this `@id` (could be
        // multiple from older deploy-server versions), then POST appends one
        // fresh route. Reuses `unregister_route` so the loop-until-404
        // sweeping logic lives in one place.
        self.unregister_route(host).await?;

        let route_id = route_id(host);
        let payload = json!({
            "@id": route_id,
            "match": [{"host": [host]}],
            "handle": [
                {
                    "handler": "reverse_proxy",
                    "upstreams": [{"dial": upstream}]
                }
            ]
        });
        let append_url = format!("{}/config/apps/http/servers/srv0/routes/...", self.base);
        let resp = self
            .http
            .post(&append_url)
            .json(&json!([payload]))
            .send()
            .await?;
        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            // Truncate the body to bound centralized-log storage if Caddy
            // ever returns a multi-KB error payload, and to limit any latent
            // secret-disclosure surface if a future caller PATCHes
            // interpolated values through this client (SEC-004). Both the
            // structured log AND the caller-facing Err use the truncated
            // form — Err is rendered into the HTTP response body via
            // IntoResponse, so we want the same 512-byte bound there.
            // `truncate_utf8` walks back to the nearest char boundary so a
            // non-ASCII body with a multi-byte codepoint straddling offset
            // 512 can't panic this path. The cap is 512 BYTES, not chars.
            let log_body = truncate_utf8(&body, 512);
            // DELETE-sweep already removed any prior route for this host, so a
            // failed POST leaves the host with ZERO routes — every request to
            // it 502s until the next successful register_route. Log loudly so
            // on-call sees the gap; the caller still gets the error.
            error!(
                host,
                upstream,
                %status,
                body = %log_body,
                body_truncated = body.len() > 512,
                "caddy register POST failed — host now has no route"
            );
            // SECURITY: DeployError::Internal renders into the HTTP response
            // body via IntoResponse, so including the Caddy admin reply here
            // exposes it to deploy-server API callers. The deploy-server
            // endpoints are admin-only (operator-facing), and Caddy admin
            // error bodies are typically short schema/config messages — no
            // user data passes through. If that trust boundary ever changes,
            // drop `log_body` from the format string and rely on the
            // structured log above.
            return Err(crate::DeployError::Internal(format!(
                "caddy register POST failed: {status} — {log_body}"
            )));
        }
        Ok(())
    }

    /// Register a host + path-prefix → upstream mapping. Sibling of
    /// `register_route` for the case where one host needs more than one
    /// route — e.g. a shared-mode worktree's `wt-<name>.<domain>` serves
    /// the frontend at `/` but must proxy `/api/*` to a separate shared
    /// backend container (fix for #453: shared worktrees previously had
    /// only the frontend route, so every `/api/*` call 502'd at the
    /// frontend dev server which has no such handler).
    ///
    /// **`suffix`** is appended to the route's `@id` so it doesn't collide
    /// with the host-only route's id. Use a stable, host-independent
    /// string per call site (e.g. `"api"`).
    ///
    /// **Caddy first-match dispatch**: routes are evaluated in array
    /// order, not by specificity. Callers MUST register the path-matched
    /// route BEFORE any host-only route on the same host, otherwise the
    /// host-only route matches first and the path matcher is never tried.
    /// `register_route(host, …)` deletes only its own `@id`, so an earlier
    /// `register_path_route` survives and stays at its earlier array
    /// position (i.e. routes end up as `[path-matched, host-only]` — the
    /// order Caddy needs).
    ///
    /// Idempotency, retry semantics, error paths, and instance assumptions
    /// match `register_route` — see that method's doc for the full
    /// rationale. `path` is the standard Caddy path matcher syntax
    /// (e.g. `"/api/*"`, `"/admin/*"`).
    #[instrument(skip(self), fields(host = %host, path = %path, upstream = %upstream, suffix = %suffix))]
    pub async fn register_path_route(
        &self,
        host: &str,
        path: &str,
        upstream: &str,
        suffix: &str,
    ) -> Result<()> {
        // DELETE-loop sweeps any stale routes carrying this `@id` first.
        self.unregister_path_route(host, suffix).await?;

        let route_id = path_route_id(host, suffix);
        let payload = json!({
            "@id": route_id,
            "match": [{"host": [host], "path": [path]}],
            "handle": [
                {
                    "handler": "reverse_proxy",
                    "upstreams": [{"dial": upstream}]
                }
            ]
        });
        let append_url = format!("{}/config/apps/http/servers/srv0/routes/...", self.base);
        let resp = self
            .http
            .post(&append_url)
            .json(&json!([payload]))
            .send()
            .await?;
        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            let log_body = truncate_utf8(&body, 512);
            error!(
                host,
                path,
                upstream,
                %status,
                body = %log_body,
                body_truncated = body.len() > 512,
                "caddy register path-route POST failed",
            );
            return Err(crate::DeployError::Internal(format!(
                "caddy register path-route POST failed: {status} — {log_body}"
            )));
        }
        Ok(())
    }

    /// Unregister a path-matched route previously added via
    /// `register_path_route` for the same `(host, suffix)` pair. Same
    /// loop-until-404 semantics as `unregister_route` but scoped to the
    /// `path_route_id(host, suffix)` namespace so the host's host-only
    /// route is untouched. 404 (route not present) is the normal "wasn't
    /// there" path and returns `Ok(())`.
    #[instrument(skip(self), fields(host = %host, suffix = %suffix))]
    pub async fn unregister_path_route(&self, host: &str, suffix: &str) -> Result<()> {
        let route_id = path_route_id(host, suffix);
        let url = format!("{}/id/{}", self.base, route_id);
        let iters = std::sync::atomic::AtomicUsize::new(0);
        let sweep = async {
            for _ in 0..MAX_DELETE_ITERS {
                let resp = self.http.delete(&url).send().await?;
                iters.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                let status = resp.status();
                if status.as_u16() == 404 {
                    return Ok(());
                }
                if !status.is_success() {
                    let body = resp.text().await.unwrap_or_default();
                    let log_body = truncate_utf8(&body, 512);
                    error!(
                        host,
                        suffix,
                        %status,
                        body = %log_body,
                        body_truncated = body.len() > 512,
                        "caddy unregister path-route DELETE failed",
                    );
                    return Err(crate::DeployError::Internal(format!(
                        "caddy unregister path-route: {status} — {log_body}"
                    )));
                }
            }
            error!(
                host,
                suffix,
                max_iters = MAX_DELETE_ITERS,
                "caddy unregister path-route: id index appears stuck",
            );
            Err(crate::DeployError::Internal(format!(
                "caddy unregister path-route: {host}/{suffix} still resolved after {MAX_DELETE_ITERS} DELETE iterations"
            )))
        };
        match tokio::time::timeout(SWEEP_DEADLINE, sweep).await {
            Ok(inner) => inner,
            Err(_) => {
                let n = iters.load(std::sync::atomic::Ordering::SeqCst);
                error!(
                    host,
                    suffix,
                    deadline_secs = SWEEP_DEADLINE.as_secs(),
                    completed_deletes = n,
                    "caddy unregister path-route: sweep deadline exceeded",
                );
                Err(crate::DeployError::Internal(format!(
                    "caddy unregister path-route: {}s deadline exceeded for {host}/{suffix} after {n} DELETEs",
                    SWEEP_DEADLINE.as_secs()
                )))
            }
        }
    }

    /// Remove ALL routes registered under this host's `@id`. Loops DELETE
    /// until Caddy returns 404, which is the documented "no such id" reply
    /// for `/id/<id>` requests. Older deploy-server versions could leave
    /// duplicates (see `register_route` doc); this sweep cleans them up.
    /// 404 on the first call is the normal "wasn't there" path and is OK.
    ///
    /// Bounded by `MAX_DELETE_ITERS` iterations AND a `SWEEP_DEADLINE` total
    /// wall-clock budget so a slow Caddy can't stall the per-worktree lock
    /// indefinitely. Both bounds surface as distinct `Err` variants whose
    /// messages identify the host and (for the deadline path) how many
    /// DELETEs completed before the budget expired.
    ///
    /// **Which bound trips first depends on per-DELETE latency** (per-request
    /// timeout is 5s — see `new`) — and the two paths return DIFFERENT errors:
    ///   - Fast Caddy (~10ms/req): the iter cap is the practical bound. All
    ///     `MAX_DELETE_ITERS` DELETEs complete inside the 15s budget; the
    ///     `for`-loop exhausts and returns the iter-count error.
    ///   - Slow Caddy (~1s/req or worse): `SWEEP_DEADLINE` trips first. The
    ///     `tokio::time::timeout` wrapping the loop returns the deadline
    ///     error (with `completed_deletes = n`) without ever entering the
    ///     exhaustion arm. `SWEEP_DEADLINE` is the load-bearing safety
    ///     bound here; the iter count is defense in depth against a Caddy
    ///     id-index bug that would otherwise let a fast loop spin forever.
    ///
    /// The two error messages are therefore precise about which bound fired:
    /// the iter-exhaustion message asserts the full `MAX_DELETE_ITERS` ran;
    /// the deadline-trip message reports the partial DELETE count.
    ///
    /// If the sweep removes any duplicates (iter > 1 before 404), a `warn!`
    /// is emitted so dashboards can alert on drift — duplicates appearing
    /// in steady-state means somebody is writing untracked routes.
    #[instrument(skip(self), fields(host = %host))]
    pub async fn unregister_route(&self, host: &str) -> Result<()> {
        let route_id = route_id(host);
        let url = format!("{}/id/{}", self.base, route_id);
        // Track iteration count outside the async block so the timeout-Err
        // branch can report progress and the success branch can warn when
        // duplicates were swept.
        let iters = std::sync::atomic::AtomicUsize::new(0);
        // Loop until 404 — older deploy-server versions appended duplicate
        // routes carrying the same `@id`, and a single DELETE only removes
        // the FIRST match. Bounded at MAX_DELETE_ITERS as a safety net so a
        // bug in Caddy's id index doesn't lock us into a hot loop.
        //
        // Total-deadline guard (SWEEP_DEADLINE): per-request timeout is 5s
        // (see `new`); MAX_DELETE_ITERS slow-but-not-timing-out iters could
        // otherwise stall ~80s while a per-worktree lock is held. Cap the
        // whole sweep so other ops on the same worktree don't starve.
        let sweep = async {
            for _ in 0..MAX_DELETE_ITERS {
                let resp = self.http.delete(&url).send().await?;
                iters.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                let status = resp.status();
                if status.as_u16() == 404 {
                    return Ok(());
                }
                if !status.is_success() {
                    // Mirror the POST-failure path in `register_route`: read
                    // the body, truncate to 512 BYTES (not chars — see
                    // `truncate_utf8`) to bound centralized-log storage and
                    // limit any latent secret-disclosure surface, and
                    // include the truncated body in both the log and the
                    // returned Err so on-call has Caddy's actual complaint,
                    // not just the bare HTTP status.
                    let body = resp.text().await.unwrap_or_default();
                    // UTF-8-safe truncation (see `truncate_utf8`): a
                    // multi-byte codepoint straddling offset 512 would
                    // otherwise panic the error-logging path.
                    let log_body = truncate_utf8(&body, 512);
                    error!(
                        host,
                        %status,
                        body = %log_body,
                        body_truncated = body.len() > 512,
                        "caddy unregister DELETE failed",
                    );
                    // SECURITY: same trade-off as the register POST path
                    // — body is echoed via IntoResponse to admin-only API
                    // callers; Caddy admin replies are operator-facing.
                    return Err(crate::DeployError::Internal(format!(
                        "caddy unregister: {status} — {log_body}"
                    )));
                }
                // 200/2xx → an entry was removed. Loop again to sweep duplicates.
            }
            error!(
                host,
                max_iters = MAX_DELETE_ITERS,
                "caddy unregister: id index appears stuck — DELETEs never returned 404",
            );
            // The deadline-trip branch returns a SEPARATE error (see the
            // `Err(_)` arm on the `tokio::time::timeout` below), so when
            // this message fires the loop ran exactly MAX_DELETE_ITERS
            // times — "after {MAX_DELETE_ITERS}" is accurate, not just
            // an upper bound.
            Err(crate::DeployError::Internal(format!(
                "caddy unregister: {host} still resolved after {MAX_DELETE_ITERS} DELETE iterations"
            )))
        };
        let result = match tokio::time::timeout(SWEEP_DEADLINE, sweep).await {
            Ok(inner) => inner,
            Err(_) => {
                let n = iters.load(std::sync::atomic::Ordering::SeqCst);
                error!(
                    host,
                    deadline_secs = SWEEP_DEADLINE.as_secs(),
                    completed_deletes = n,
                    "caddy unregister: sweep deadline exceeded",
                );
                Err(crate::DeployError::Internal(format!(
                    "caddy unregister: {}s deadline exceeded for {host} after {n} DELETEs",
                    SWEEP_DEADLINE.as_secs()
                )))
            }
        };
        // Drift signal: warn ONLY when we removed MORE than one prior route.
        // `iters` counts every completed DELETE including the terminal 404,
        // so `swept = iters - 1` is the count of 2xx DELETEs. One 2xx is the
        // expected steady state (register_route calls unregister_route first
        // to sweep its own prior entry); two or more means somebody else
        // wrote a route under our `@id`, which is what we want paged on.
        if result.is_ok() {
            let n = iters.load(std::sync::atomic::Ordering::SeqCst);
            let swept = n.saturating_sub(1);
            if swept > 1 {
                warn!(
                    host,
                    swept, "caddy: removed stale duplicate routes (drift signal)",
                );
            }
        }
        result
    }

    /// Cheap GET against the Caddy admin root so the preflight validator
    /// (see `infra/preflight.rs`) can fail fast on a wedged or paused Caddy
    /// instead of letting `register_route` hang on the first `connect_timeout`.
    pub async fn ping(&self) -> Result<()> {
        let url = format!("{}/config/", self.base);
        let resp = self.http.get(&url).send().await?;
        if !resp.status().is_success() {
            return Err(crate::DeployError::Internal(format!(
                "caddy ping: {}",
                resp.status()
            )));
        }
        Ok(())
    }

    /// One-shot startup reconciliation: list every route under
    /// `apps.http.servers.srv0.routes` and DELETE any whose first matched
    /// host starts with `host_prefix` but whose `@id` is missing or doesn't
    /// start with `ppt-deploy-`.
    ///
    /// This exists to clean up routes left by older deploy-server versions
    /// that POST-appended without setting `@id`. The DELETE-by-id sweep in
    /// `unregister_route` can't see them — `register_route` would happily
    /// add a tagged duplicate alongside the stale untagged one and Caddy's
    /// first-match dispatch would keep routing to the dead container.
    ///
    /// Returns the number of routes deleted. Safe to call repeatedly —
    /// once all legacy untagged routes have been swept, subsequent calls
    /// are no-ops. Logs at `info!` when count > 0, `warn!` on individual
    /// per-route DELETE failures (and continues with the rest).
    ///
    /// DELETEs are issued by array index in descending order so an early
    /// DELETE doesn't shift the indices of later targets. Caddy admin
    /// serializes config writes so concurrent unrelated route additions
    /// during this call won't race us (and a fresh untagged route is
    /// virtually impossible after this fix ships).
    #[instrument(skip(self), fields(host_prefix = %host_prefix))]
    pub async fn purge_legacy_untagged_routes(&self, host_prefix: &str) -> Result<usize> {
        let list_url = format!("{}/config/apps/http/servers/srv0/routes", self.base);
        let resp = self.http.get(&list_url).send().await?;
        let status = resp.status();
        if status.as_u16() == 404 {
            // No srv0 yet — fresh Caddy with no routes provisioned. Nothing
            // to do.
            return Ok(0);
        }
        if !status.is_success() {
            return Err(crate::DeployError::Internal(format!(
                "caddy purge: GET routes failed: {status}"
            )));
        }
        let routes: serde_json::Value = resp.json().await?;
        let arr = match routes.as_array() {
            Some(a) => a,
            // Caddy returns `null` when the array is empty (no routes ever
            // added). Treat as "nothing to purge".
            None => return Ok(0),
        };
        let mut indices_to_delete: Vec<usize> = arr
            .iter()
            .enumerate()
            .filter_map(|(i, r)| {
                let host = r
                    .get("match")?
                    .as_array()?
                    .first()?
                    .get("host")?
                    .as_array()?
                    .first()?
                    .as_str()?;
                if !host.starts_with(host_prefix) {
                    return None;
                }
                let id_ok = r
                    .get("@id")
                    .and_then(|v| v.as_str())
                    .map(|id| id.starts_with("ppt-deploy-"))
                    .unwrap_or(false);
                if id_ok {
                    None
                } else {
                    Some(i)
                }
            })
            .collect();

        if indices_to_delete.is_empty() {
            return Ok(0);
        }

        // Sort descending so we DELETE from the tail forward; Caddy shifts
        // remaining entries down on each DELETE, so popping earlier indices
        // first would invalidate the later ones.
        indices_to_delete.sort_unstable_by(|a, b| b.cmp(a));

        let mut deleted = 0usize;
        for idx in indices_to_delete {
            let del_url = format!("{}/config/apps/http/servers/srv0/routes/{}", self.base, idx);
            match self.http.delete(&del_url).send().await {
                Ok(r) if r.status().is_success() => {
                    deleted += 1;
                }
                Ok(r) => {
                    warn!(
                        idx,
                        status = %r.status(),
                        "caddy purge: DELETE route by index failed; continuing",
                    );
                }
                Err(e) => {
                    warn!(idx, error = %e, "caddy purge: DELETE transport error; continuing");
                }
            }
        }
        if deleted > 0 {
            info!(
                deleted,
                host_prefix, "caddy purge: removed legacy untagged routes",
            );
        }
        Ok(deleted)
    }
}

fn sanitize_id(host: &str) -> String {
    host.chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect()
}

/// Stable `@id` scheme for ppt-deploy-managed Caddy routes. Single source
/// of truth — both `register_route` (POST payload) and `unregister_route`
/// (DELETE URL) compute the id through here.
fn route_id(host: &str) -> String {
    format!("ppt-deploy-{}", sanitize_id(host))
}

/// `@id` for a path-matched route that lives alongside a host-only route
/// for the same host. The suffix keeps the two ids distinct so each can be
/// added/removed independently (e.g. shared-mode worktree `/api/*` route
/// added before the host-only frontend route — fix for #453).
fn path_route_id(host: &str, suffix: &str) -> String {
    format!("ppt-deploy-{}-{}", sanitize_id(host), suffix)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Spin up a tiny in-process axum server that records the arrival order
    /// of every request (method) and lets the caller program the DELETE
    /// status sequence AND the POST status. Returns `(addr, log, server_task)`.
    /// Caller aborts `server_task` after asserting on `log`.
    ///
    /// Why bespoke instead of httpmock: httpmock 0.7 doesn't expose request
    /// arrival order or stateful response sequences, both of which we need
    /// to assert that DELETE strictly precedes POST and that the DELETE
    /// loop terminates exactly when 404 arrives.
    async fn spawn_caddy_stub(
        delete_status_sequence: Vec<u16>,
        post_status: u16,
    ) -> (
        std::net::SocketAddr,
        std::sync::Arc<std::sync::Mutex<Vec<&'static str>>>,
        tokio::task::JoinHandle<()>,
    ) {
        use std::sync::atomic::{AtomicUsize, Ordering};
        use std::sync::{Arc, Mutex};

        let log: Arc<Mutex<Vec<&'static str>>> = Arc::new(Mutex::new(Vec::new()));
        let delete_idx = Arc::new(AtomicUsize::new(0));
        let delete_seq = Arc::new(delete_status_sequence);

        let log_d = log.clone();
        let log_p = log.clone();
        let di = delete_idx.clone();
        let ds = delete_seq.clone();

        let app = axum::Router::new()
            .route(
                "/id/{*tail}",
                axum::routing::delete(move || {
                    let log = log_d.clone();
                    let idx = di.clone();
                    let seq = ds.clone();
                    async move {
                        log.lock().unwrap().push("DELETE");
                        let n = idx.fetch_add(1, Ordering::SeqCst);
                        let status = seq.get(n).copied().unwrap_or(404);
                        axum::http::StatusCode::from_u16(status).unwrap()
                    }
                }),
            )
            .route(
                "/config/apps/http/servers/srv0/routes/...",
                axum::routing::post(move || {
                    let log = log_p.clone();
                    async move {
                        log.lock().unwrap().push("POST");
                        axum::http::StatusCode::from_u16(post_status).unwrap()
                    }
                }),
            );

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let server_task = tokio::spawn(async move {
            // .ok() — `task.abort()` cancels the accept loop and trips an
            // error on shutdown that's not interesting to surface in tests.
            axum::serve(listener, app).await.ok();
        });
        (addr, log, server_task)
    }

    #[tokio::test]
    async fn register_route_does_delete_strictly_before_post() {
        // Happy path: no stale routes. DELETE returns 404 immediately, then
        // POST appends one fresh route. Asserts the EXACT arrival order
        // (`["DELETE", "POST"]`) so a refactor that swapped the calls would
        // fail loudly — not just hit counts that swapping would still pass.
        let (addr, log, task) = spawn_caddy_stub(vec![404], 200).await;
        let client = CaddyClient::new(format!("http://{addr}"));
        client
            .register_route("wt-uc14.dev.ppt.rlt.sk", "127.0.0.1:51001")
            .await
            .unwrap();

        let final_log = log.lock().unwrap().clone();
        assert_eq!(
            final_log,
            vec!["DELETE", "POST"],
            "happy path must be DELETE then POST in that order"
        );
        task.abort();
    }

    #[tokio::test]
    async fn register_route_sweeps_existing_duplicates_then_posts_once() {
        // Caddy holds two stale duplicate routes for this `@id` (older
        // deploy-server versions could leave them). The DELETE loop must
        // run three times — 200, 200, 404 — before a single POST appends
        // the fresh route. Order matters, so we assert the entire log.
        let (addr, log, task) = spawn_caddy_stub(vec![200, 200, 404], 200).await;
        let client = CaddyClient::new(format!("http://{addr}"));
        client
            .register_route("dup.example.com", "127.0.0.1:9999")
            .await
            .unwrap();

        let final_log = log.lock().unwrap().clone();
        assert_eq!(
            final_log,
            vec!["DELETE", "DELETE", "DELETE", "POST"],
            "loop must DELETE until 404, then POST once"
        );
        task.abort();
    }

    #[tokio::test]
    async fn unregister_404_is_ok() {
        // Bare unregister of a host with no route — DELETE returns 404 and
        // that's the documented "wasn't there" success path.
        let (addr, log, task) = spawn_caddy_stub(vec![404], 200).await;
        let client = CaddyClient::new(format!("http://{addr}"));
        client.unregister_route("missing.dev.rlt.sk").await.unwrap();
        assert_eq!(*log.lock().unwrap(), vec!["DELETE"]);
        task.abort();
    }

    #[tokio::test]
    async fn register_route_post_failure_returns_err_after_delete_sweep() {
        // DELETE-sweep succeeds (404 — nothing there), then Caddy returns
        // 500 on the POST. Caller must see Err; log will show that DELETE
        // ran before POST. This pins ERR-001: a failed POST is observable.
        let (addr, log, task) = spawn_caddy_stub(vec![404], 500).await;
        let client = CaddyClient::new(format!("http://{addr}"));
        let err = client
            .register_route("postfail.example.com", "127.0.0.1:9000")
            .await
            .expect_err("POST 500 must surface as Err");
        assert!(
            err.to_string().contains("caddy register POST failed"),
            "unexpected error: {err}"
        );
        // Pin the variant alongside the message so a future refactor to a
        // typed CaddyError can't silently pass the substring check (ERR2-001).
        assert!(
            matches!(err, crate::DeployError::Internal(_)),
            "expected DeployError::Internal, got {err:?}"
        );
        assert_eq!(*log.lock().unwrap(), vec!["DELETE", "POST"]);
        task.abort();
    }

    #[tokio::test]
    async fn unregister_route_exhaustion_returns_err() {
        // Pathological Caddy that NEVER returns 404 — the MAX_DELETE_ITERS
        // safety net must bail with a clear error mentioning the host.
        // Program 17 consecutive 200s so the loop runs to its full bound.
        // Also assert the loop actually iterated MAX_DELETE_ITERS times —
        // a regression that bailed early with the same wording would
        // otherwise sneak past a message-only check (COMP2-002).
        let (addr, log, task) = spawn_caddy_stub(vec![200; MAX_DELETE_ITERS + 1], 200).await;
        let client = CaddyClient::new(format!("http://{addr}"));
        let err = client
            .unregister_route("stuck.example.com")
            .await
            .expect_err("exhaustion must surface as Err");
        let msg = err.to_string();
        assert!(
            msg.contains("stuck.example.com")
                && msg.contains(&format!("after {MAX_DELETE_ITERS} DELETE iterations")),
            "unexpected exhaustion error: {msg}"
        );
        // Pin the variant alongside the message so a future refactor to a
        // typed CaddyError can't silently pass the substring check (ERR2-001).
        assert!(
            matches!(err, crate::DeployError::Internal(_)),
            "expected DeployError::Internal, got {err:?}"
        );
        assert_eq!(
            log.lock().unwrap().len(),
            MAX_DELETE_ITERS,
            "loop must run exactly MAX_DELETE_ITERS times before bailing"
        );
        task.abort();
    }

    /// Bespoke stub for the body-on-DELETE-failure test: returns a fixed
    /// status + body on every DELETE. The shared `spawn_caddy_stub` returns
    /// bare `StatusCode` (empty body), which can't exercise the new code
    /// path that surfaces Caddy's complaint in the log + Err.
    async fn spawn_delete_body_stub(
        status: u16,
        body: &'static str,
    ) -> (std::net::SocketAddr, tokio::task::JoinHandle<()>) {
        let app = axum::Router::new().route(
            "/id/{*tail}",
            axum::routing::delete(move || async move {
                (axum::http::StatusCode::from_u16(status).unwrap(), body)
            }),
        );
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let server_task = tokio::spawn(async move {
            axum::serve(listener, app).await.ok();
        });
        (addr, server_task)
    }

    #[tokio::test]
    async fn unregister_route_delete_500_logs_body_and_returns_err() {
        // Caddy returns 500 with an informative body on DELETE. The Err
        // must carry the body substring (not just the bare HTTP status)
        // so on-call sees Caddy's actual complaint without grepping the
        // structured log. Pins the body-in-Err half of the Copilot #2 fix;
        // the log-with-body half is verified by code review of the
        // structured `error!` call (no tracing harness in this crate).
        const BODY: &str = "unknown id: caddy lost its index";
        let (addr, task) = spawn_delete_body_stub(500, BODY).await;
        let client = CaddyClient::new(format!("http://{addr}"));
        let err = client
            .unregister_route("dies.example.com")
            .await
            .expect_err("DELETE 500 must surface as Err");
        let msg = err.to_string();
        assert!(
            msg.contains(BODY),
            "Err must include response body for on-call diagnostics: {msg}"
        );
        assert!(
            msg.contains("500"),
            "Err must still include the HTTP status: {msg}"
        );
        task.abort();
    }

    #[tokio::test]
    async fn unregister_route_propagates_non_404_delete_failure() {
        // DELETE returns 500 on the first iteration → loop must bail
        // immediately with an Err carrying the upstream status. Pins
        // the `!status.is_success()` branch (COMP2-004).
        let (addr, log, task) = spawn_caddy_stub(vec![500], 200).await;
        let client = CaddyClient::new(format!("http://{addr}"));
        let err = client
            .unregister_route("broken.example.com")
            .await
            .expect_err("DELETE 500 must surface as Err");
        let msg = err.to_string();
        assert!(
            msg.contains("caddy unregister") && msg.contains("500"),
            "unexpected DELETE-failure error: {msg}"
        );
        // Pin the variant alongside the message so a future refactor to a
        // typed CaddyError can't silently pass the substring check (ERR2-001).
        assert!(
            matches!(err, crate::DeployError::Internal(_)),
            "expected DeployError::Internal, got {err:?}"
        );
        assert_eq!(
            log.lock().unwrap().len(),
            1,
            "must bail after the first failed DELETE, not retry"
        );
        task.abort();
    }

    #[test]
    fn truncate_utf8_never_panics_on_multibyte_boundary() {
        // ASCII passthrough under the cap.
        assert_eq!(truncate_utf8("abc", 512), "abc");
        // Bare byte-slice would panic here: each 'é' is 2 bytes, so a
        // cap of 3 lands inside the second codepoint. `truncate_utf8`
        // walks back to byte 2 (after the first 'é').
        assert_eq!(truncate_utf8("éé", 3), "é");
        // 4-byte codepoint (😀 = 0xF0 0x9F 0x98 0x80) straddling the
        // cap — must walk back to 0 rather than slice mid-codepoint.
        assert_eq!(truncate_utf8("😀", 2), "");
        // Cap exactly on a char boundary keeps the whole prefix.
        assert_eq!(truncate_utf8("héllo", 3), "hé");
    }

    #[test]
    fn route_id_uses_stable_prefix_and_sanitizes_host() {
        // Pin the `@id` scheme — both register_route POST payload and
        // unregister_route DELETE URL compute through this single helper,
        // so a drift here would silently orphan every existing route.
        assert_eq!(
            route_id("wt-uc14.dev.ppt.rlt.sk"),
            "ppt-deploy-wt-uc14-dev-ppt-rlt-sk"
        );
        assert_eq!(route_id("simple"), "ppt-deploy-simple");
    }

    /// Stub for `purge_legacy_untagged_routes`: serves a programmable
    /// routes JSON on GET and records the indices DELETEd. Bespoke
    /// because `spawn_caddy_stub` is shaped for the DELETE-by-id pattern
    /// of register/unregister, not the index-DELETE pattern of purge.
    async fn spawn_purge_stub(
        routes_json: serde_json::Value,
    ) -> (
        std::net::SocketAddr,
        std::sync::Arc<std::sync::Mutex<Vec<usize>>>,
        tokio::task::JoinHandle<()>,
    ) {
        use axum::extract::Path;
        use std::sync::{Arc, Mutex};

        let deleted: Arc<Mutex<Vec<usize>>> = Arc::new(Mutex::new(Vec::new()));
        let routes = Arc::new(routes_json);

        let routes_g = routes.clone();
        let deleted_d = deleted.clone();

        let app = axum::Router::new()
            .route(
                "/config/apps/http/servers/srv0/routes",
                axum::routing::get(move || {
                    let routes = routes_g.clone();
                    async move { axum::Json((*routes).clone()) }
                }),
            )
            .route(
                "/config/apps/http/servers/srv0/routes/{idx}",
                axum::routing::delete(move |Path(idx): Path<usize>| {
                    let deleted = deleted_d.clone();
                    async move {
                        deleted.lock().unwrap().push(idx);
                        axum::http::StatusCode::OK
                    }
                }),
            );

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let server_task = tokio::spawn(async move {
            axum::serve(listener, app).await.ok();
        });
        (addr, deleted, server_task)
    }

    #[tokio::test]
    async fn purge_legacy_untagged_routes_removes_only_matching_untagged() {
        // 4 routes:
        //   [0] wt-old.dev — NO @id          → MUST be deleted (target)
        //   [1] api.prod   — has ppt-deploy- → unrelated, must NOT touch
        //   [2] wt-tagged  — has ppt-deploy- → tagged, must NOT touch
        //   [3] wt-foreign — has @id "foo"   → not ours, MUST be deleted
        // Expected DELETE order: [3, 0] (descending so earlier indices
        // don't shift the remaining targets).
        let routes = serde_json::json!([
            { "match": [{"host": ["wt-old.dev.ppt.rlt.sk"]}], "handle": [] },
            { "@id": "ppt-deploy-api-prod", "match": [{"host": ["api.prod.example.com"]}], "handle": [] },
            { "@id": "ppt-deploy-wt-tagged", "match": [{"host": ["wt-tagged.dev.rlt.sk"]}], "handle": [] },
            { "@id": "foo", "match": [{"host": ["wt-foreign.dev.rlt.sk"]}], "handle": [] }
        ]);
        let (addr, deleted, task) = spawn_purge_stub(routes).await;
        let client = CaddyClient::new(format!("http://{addr}"));
        let n = client.purge_legacy_untagged_routes("wt-").await.unwrap();
        assert_eq!(
            n, 2,
            "must delete only the 2 wt-* untagged/foreign-id routes"
        );
        assert_eq!(
            *deleted.lock().unwrap(),
            vec![3, 0],
            "must DELETE in descending index order so shifts don't invalidate later targets",
        );
        task.abort();
    }

    #[tokio::test]
    async fn purge_legacy_untagged_routes_no_op_on_clean_caddy() {
        // All routes already carry our @id → zero deletions, zero churn.
        let routes = serde_json::json!([
            { "@id": "ppt-deploy-a", "match": [{"host": ["wt-a.dev"]}], "handle": [] },
            { "@id": "ppt-deploy-b", "match": [{"host": ["wt-b.dev"]}], "handle": [] }
        ]);
        let (addr, deleted, task) = spawn_purge_stub(routes).await;
        let client = CaddyClient::new(format!("http://{addr}"));
        let n = client.purge_legacy_untagged_routes("wt-").await.unwrap();
        assert_eq!(n, 0);
        assert!(deleted.lock().unwrap().is_empty());
        task.abort();
    }
}
