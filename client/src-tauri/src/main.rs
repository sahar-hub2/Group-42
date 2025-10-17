// GROUP: 42
// MEMBERS: Ray Okamoto, Phoenix Pereira, Kayla Rowley, Qi Wu, Ho Yin Li

// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use axum::{extract::Query, routing::get, Json, Router};
use clap::{ArgAction, Parser};
use serde::Deserialize;
use std::net::SocketAddr;
use std::time::Duration;
use tower_http::cors::{Any, CorsLayer};

const DEFAULT_WEB_HOST: &str = "localhost";
const DEFAULT_WEB_PORT: u16 = 1420;
const DEFAULT_API_HOST: &str = "127.0.0.1";
const DEFAULT_API_PORT: u16 = 3000;

/// Secure Chat App
#[derive(Parser, Debug)]
#[command(author, version, about)]
struct Cli {
    /// Run in CLI mode (no desktop window). Starts an HTTP API server and opens the browser UI.
    #[arg(long, action = ArgAction::SetTrue)]
    cli: bool,

    /// Web UI host
    #[arg(long, default_value = DEFAULT_WEB_HOST)]
    web_host: String,

    /// Web UI port
    #[arg(long, default_value_t = DEFAULT_WEB_PORT)]
    web_port: u16,

    /// API server host (served by this binary)
    #[arg(long, default_value = DEFAULT_API_HOST)]
    api_host: String,

    /// API server port (served by this binary)
    #[arg(long, default_value_t = DEFAULT_API_PORT)]
    api_port: u16,

    /// URL to open in the browser (overrides web_host/web_port)
    #[arg(long)]
    open_url: Option<String>,
}

#[tokio::main]
async fn main() {
    let args = Cli::parse();

    if args.cli {
        // Build API router
        #[derive(Deserialize)]
        struct GreetQuery {
            name: String,
        }
        let cors = CorsLayer::new()
            .allow_origin(Any)
            .allow_methods(Any)
            .allow_headers(Any);

        let app = Router::new()
            .route(
                "/api/greet",
                get(|Query(GreetQuery { name })| async move {
                    let res = shared::greet(&name);
                    Json(res)
                }),
            )
            .layer(cors);

        // Start the API server
        let api_addr: SocketAddr = format!("{}:{}", args.api_host, args.api_port)
            .parse()
            .expect("invalid API host/port");
        let listener = tokio::net::TcpListener::bind(api_addr)
            .await
            .expect("failed to bind API address");
        let server = tokio::spawn(async move {
            axum::serve(listener, app).await.expect("API server failed");
        });

        // Give the server a brief moment to start
        tokio::time::sleep(Duration::from_millis(300)).await;

        // Open browser to the web UI
        let url = args
            .open_url
            .unwrap_or_else(|| format!("http://{}:{}", args.web_host, args.web_port));
        let _ = webbrowser::open(&url);

        // Exit on Ctrl-C
        let _ = ctrlc::set_handler(move || {
            std::process::exit(0);
        });

        // Run until server ends
        let _ = server.await;
    } else {
        // Normal Tauri desktop run
        secure_chat_lib::run()
    }
}
