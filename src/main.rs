//! Binary entry: seeded synthetic patients, serve.

use patient_360_dashboard::routes;
use patient_360_dashboard::state::AppState;
use tower_http::trace::TraceLayer;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,patient_360_dashboard=debug".into()),
        )
        .init();

    let port: u16 = std::env::var("APP_PORT")
        .unwrap_or_else(|_| "8007".to_string())
        .parse()
        .expect("APP_PORT must be a valid port");

    let app = routes::router(AppState::seeded()).layer(TraceLayer::new_for_http());
    let listener = tokio::net::TcpListener::bind(("0.0.0.0", port)).await?;
    tracing::info!(%port, "patient 360 dashboard listening");
    axum::serve(listener, app).await?;
    Ok(())
}
