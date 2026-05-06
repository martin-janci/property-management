// backend/servers/deploy-server/src/bin/pmctl.rs
use anyhow::Context;
use clap::{Parser, Subcommand};
use serde::Serialize;

#[derive(Parser)]
#[command(name = "pmctl", version)]
struct Cli {
    /// Deploy server base URL (default: from $PPT_DEPLOY_URL or https://deploy.rlt.sk)
    #[arg(long, env = "PPT_DEPLOY_URL", default_value = "https://deploy.rlt.sk")]
    url: String,
    /// API token (default: from $PPT_DEPLOY_TOKEN or ~/.config/ppt-deploy/token)
    #[arg(long, env = "PPT_DEPLOY_TOKEN")]
    token: Option<String>,
    /// Output JSON instead of human-readable.
    #[arg(long, global = true)]
    json: bool,
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Open a worktree.
    Open {
        branch: String,
        #[arg(long)]
        alias: Option<String>,
        #[arg(long, default_value = "shared")]
        backend: String,
        #[arg(long)]
        ttl: Option<i64>,
    },
    /// Close a worktree (graceful).
    Close {
        name: String,
        #[arg(long)]
        hard: bool,
    },
    /// Show worktree status.
    Status { name: Option<String> },
    /// List all worktrees.
    List,
    /// Print version of the server.
    Version,
    /// Deploy a tag to a target (staging only in Phase 2).
    Deploy {
        target: String,
        #[arg(long)]
        tag: String,
    },
    /// Resume a paused target on demand.
    Wake { target: String },
    /// Stream container logs.
    Logs {
        name: String,
        #[arg(short = 'f', long)]
        follow: bool,
        #[arg(long)]
        service: Option<String>,
    },
}

#[derive(Serialize)]
struct OpenBody<'a> {
    branch: &'a str,
    alias: Option<&'a String>,
    backend: &'a str,
    ttl_seconds: Option<i64>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let token = match cli.token {
        Some(t) => t,
        None => std::fs::read_to_string(dirs::config_dir().unwrap().join("ppt-deploy/token"))
            .context("read ~/.config/ppt-deploy/token")?
            .trim()
            .to_string(),
    };

    let http = reqwest::Client::new();
    let auth = format!("Bearer {token}");

    match cli.cmd {
        Cmd::Open {
            branch,
            alias,
            backend,
            ttl,
        } => {
            let body = OpenBody {
                branch: &branch,
                alias: alias.as_ref(),
                backend: &backend,
                ttl_seconds: ttl,
            };
            let resp = http
                .post(format!("{}/api/worktree", cli.url))
                .header("Authorization", &auth)
                .json(&body)
                .send()
                .await?;
            print_resp(resp, cli.json).await?;
        }
        Cmd::Close { name, hard: _ } => {
            // Phase 1 ignores --hard (Phase 3 will add it).
            let resp = http
                .post(format!("{}/api/worktree/{name}/close", cli.url))
                .header("Authorization", &auth)
                .send()
                .await?;
            print_resp(resp, cli.json).await?;
        }
        Cmd::Status { name } => {
            let url = match name {
                Some(n) => format!("{}/api/worktree/{n}", cli.url),
                None => format!("{}/api/worktrees", cli.url),
            };
            let resp = http.get(url).header("Authorization", &auth).send().await?;
            print_resp(resp, cli.json).await?;
        }
        Cmd::List => {
            let resp = http
                .get(format!("{}/api/worktrees", cli.url))
                .header("Authorization", &auth)
                .send()
                .await?;
            print_resp(resp, cli.json).await?;
        }
        Cmd::Version => {
            let resp = http.get(format!("{}/health", cli.url)).send().await?;
            print_resp(resp, cli.json).await?;
        }
        Cmd::Deploy { target, tag } => {
            let body = serde_json::json!({"tag": tag, "target": target});
            let resp = http
                .post(format!("{}/api/deploy", cli.url))
                .header("Authorization", &auth)
                .json(&body)
                .send()
                .await?;
            print_resp(resp, cli.json).await?;
        }
        Cmd::Wake { target } => {
            let resp = http
                .post(format!("{}/api/wake/{target}", cli.url))
                .header("Authorization", &auth)
                .send()
                .await?;
            print_resp(resp, cli.json).await?;
        }
        Cmd::Logs {
            name,
            follow,
            service,
        } => {
            let mut url = format!("{}/api/logs/{name}?follow={follow}", cli.url);
            if let Some(s) = service {
                url.push_str(&format!("&service={s}"));
            }
            let resp = http
                .get(&url)
                .header("Authorization", &auth)
                .send()
                .await?
                .error_for_status()?;
            let mut stream = resp.bytes_stream();
            use futures_util::StreamExt;
            while let Some(chunk) = stream.next().await {
                if let Ok(bytes) = chunk {
                    // Parse SSE — each event is `data: <line>\n\n`.
                    let s = String::from_utf8_lossy(&bytes);
                    for line in s.lines() {
                        if let Some(payload) = line.strip_prefix("data: ") {
                            println!("{payload}");
                        }
                    }
                }
            }
        }
    }
    Ok(())
}

async fn print_resp(resp: reqwest::Response, as_json: bool) -> anyhow::Result<()> {
    let status = resp.status();
    let text = resp.text().await?;
    if as_json {
        println!("{text}");
    } else if status.is_success() {
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(&text) {
            println!("{}", serde_json::to_string_pretty(&v)?);
        } else {
            println!("{text}");
        }
    } else {
        anyhow::bail!("HTTP {}: {}", status, text);
    }
    Ok(())
}
