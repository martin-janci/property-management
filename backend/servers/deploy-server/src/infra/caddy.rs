// backend/servers/deploy-server/src/infra/caddy.rs
use crate::Result;
use serde_json::json;

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
    /// with exactly one route for `host`, regardless of how many times it's
    /// called or what the prior `@id` index state was.
    ///
    /// Strategy: DELETE-by-id (looped — see `unregister_route`) first, then
    /// POST-append a fresh route to `apps.http.servers.srv0.routes`.
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
    /// the array contains exactly one entry per host after this call.
    pub async fn register_route(&self, host: &str, upstream: &str) -> Result<()> {
        // DELETE-loop sweeps any stale routes carrying this `@id` (could be
        // multiple from older deploy-server versions), then POST appends one
        // fresh route. Reuses `unregister_route` so the loop-until-404
        // sweeping logic lives in one place.
        self.unregister_route(host).await?;

        let route_id = format!("ppt-deploy-{}", sanitize_id(host));
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
            return Err(crate::DeployError::Internal(format!(
                "caddy register POST failed: {status} — {body}"
            )));
        }
        Ok(())
    }

    /// Remove ALL routes registered under this host's `@id`. Loops DELETE
    /// until Caddy returns 404, which is the documented "no such id" reply
    /// for `/id/<id>` requests. Older deploy-server versions could leave
    /// duplicates (see `register_route` doc); this sweep cleans them up.
    /// 404 on the first call is the normal "wasn't there" path and is OK.
    pub async fn unregister_route(&self, host: &str) -> Result<()> {
        let route_id = format!("ppt-deploy-{}", sanitize_id(host));
        let url = format!("{}/id/{}", self.base, route_id);
        // Loop until 404 — older deploy-server versions appended duplicate
        // routes carrying the same `@id`, and a single DELETE only removes
        // the FIRST match. Bounded at 16 iterations as a safety net so a
        // bug in Caddy's id index doesn't lock us into a hot loop.
        for _ in 0..16 {
            let resp = self.http.delete(&url).send().await?;
            let status = resp.status();
            if status.as_u16() == 404 {
                return Ok(());
            }
            if !status.is_success() {
                return Err(crate::DeployError::Internal(format!(
                    "caddy unregister: {status}"
                )));
            }
            // 200/2xx → an entry was removed. Loop again to sweep duplicates.
        }
        Err(crate::DeployError::Internal(format!(
            "caddy unregister: {host} still resolved after 16 DELETE iterations"
        )))
    }

    /// Probe the Caddy admin API. Returns `Ok(())` when `/config/` responds
    /// with any 2xx — empty config is fine, that's the cold-start state on
    /// a freshly-bootstrapped target. Used by the preflight validator so a
    /// dead-or-paused Caddy fails fast instead of letting `register_route`
    /// hang on the first `connect_timeout`.
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
}

fn sanitize_id(host: &str) -> String {
    host.chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use httpmock::prelude::*;

    /// Spin up a tiny in-process axum server that records the arrival order
    /// of every request (method) and lets the caller program the DELETE
    /// status sequence. Returns `(addr, log, server_task)`. Caller aborts
    /// `server_task` after asserting on `log`.
    ///
    /// Why bespoke instead of httpmock: httpmock 0.7 doesn't expose request
    /// arrival order or stateful response sequences, both of which we need
    /// to assert that DELETE strictly precedes POST and that the DELETE
    /// loop terminates exactly when 404 arrives.
    async fn spawn_caddy_stub(
        delete_status_sequence: Vec<u16>,
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
                "/id/*tail",
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
                        axum::http::StatusCode::OK
                    }
                }),
            );

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let server_task = tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });
        (addr, log, server_task)
    }

    #[tokio::test]
    async fn register_route_does_delete_strictly_before_post() {
        // Happy path: no stale routes. DELETE returns 404 immediately, then
        // POST appends one fresh route. Asserts the EXACT arrival order
        // (`["DELETE", "POST"]`) so a refactor that swapped the calls would
        // fail loudly — not just hit counts that swapping would still pass.
        let (addr, log, task) = spawn_caddy_stub(vec![404]).await;
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
        let (addr, log, task) = spawn_caddy_stub(vec![200, 200, 404]).await;
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
        let server = MockServer::start();
        server.mock(|when, then| {
            when.method(DELETE).path_contains("/id/ppt-deploy-");
            then.status(404);
        });
        let client = CaddyClient::new(server.base_url());
        client.unregister_route("missing.dev.rlt.sk").await.unwrap();
    }
}
