mod github_app_auth;
mod routes;

use axum::{Router, routing::{get, post}};
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("forge_api=info".parse().expect("valid directive"))
                .add_directive("tower_http=debug".parse().expect("valid directive")),
        )
        .init();

    let app = Router::new()
        .route("/health", get(routes::health::handler))
        .route("/api/github/webhook", post(routes::github_app::handler))
        .route("/api/run", post(routes::run::handler))
        .route("/api/issues", get(routes::issues::handler))
        .route("/api/stats", get(routes::stats::handler))
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http());

    let port = std::env::var("FORGE_API_PORT")
        .ok()
        .and_then(|p| p.parse::<u16>().ok())
        .unwrap_or(5000);

    let addr = format!("0.0.0.0:{port}");
    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .unwrap_or_else(|e| panic!("Failed to bind {addr}: {e}"));

    tracing::info!("forge-api listening on {addr}");
    axum::serve(listener, app)
        .await
        .expect("server error");
}
