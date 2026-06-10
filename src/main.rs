use clap::Parser;
use tracing_subscriber::EnvFilter;

mod config;
mod db;
mod providers;
mod routes;

#[derive(Parser)]
#[command(name = "open-provider", about = "AI provider model catalog monitor")]
struct Cli {
    /// Config file path
    #[arg(short, long, default_value = "open-provider.toml")]
    config: String,

    /// Bind address
    #[arg(short, long, default_value = "127.0.0.1:3000")]
    bind: String,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .init();

    let cli = Cli::parse();
    let cfg = config::load(&cli.config)?;
    let db = db::init(&cfg.database_path)?;

    tracing::info!("open-provider starting on {}", cli.bind);

    let app = routes::app(db, cfg.clone());
    let listener = tokio::net::TcpListener::bind(&cli.bind).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
