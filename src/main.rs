use std::sync::Arc;

use clap::Parser;
use dotenvy::dotenv;
use tokio::net::TcpListener;
use tracing_bunyan_formatter::BunyanFormattingLayer;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::EnvFilter;

use self::routes::init_router;
use self::startgg::oauth::OAuthConfig;

mod database;
mod discord;
mod omegastrikers;
mod routes;
mod startgg;
mod views;

const IDENTITY_FILE: &str = "/home/roberto/.local/share/Steam/steamapps/compatdata/1869590/pfx/drive_c/users/steamuser/AppData/Local/OmegaStrikers/identity.json";

#[derive(clap::Parser, Debug)]
#[command(version, about, long_about = None)]
struct App {
    #[arg(long, env = "APP_NAME", default_value = "omega-championship")]
    app_name: String,
    #[arg(long, env = "LISTEN_ADDR", default_value = "127.0.0.1:3000")]
    listen_address: String,
    #[arg(long, env = "DATABASE_URL")]
    db_url: String,
    #[arg(long, env = "DISCORD_BOT_TOKEN")]
    discord_bot_token: String,
    #[arg(long, env = "STARTGG_OAUTH_CLIENT_ID")]
    startgg_oauth_client_id: String,
    #[arg(long, env = "STARTGG_OAUTH_CLIENT_SECRET")]
    startgg_oauth_client_secret: String,
    #[arg(long, env = "STARTGG_TOKEN")]
    startgg_token: String,
    #[arg(long, default_value = "http://127.0.0.1:3000/oauth/startgg_callback")]
    startgg_redirect_uri: String,

    #[arg(long, env = "COOKIE_SIGNING_KEY")]
    cookie_key: String,

    #[arg(long, env = "OMEGASTRIKERS_IDENTITY_FILE", default_value = IDENTITY_FILE)]
    omegastrikers_identity_file: String,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    _ = dotenv(); // ignore errors if there is no .env file

    let app = Arc::new(App::parse());

    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    let formatting_layer = BunyanFormattingLayer::new(app.app_name.clone(), std::io::stdout);

    let subscriber = tracing_subscriber::Registry::default()
        .with(env_filter)
        .with(tracing_bunyan_formatter::JsonStorageLayer)
        .with(formatting_layer);

    tracing::subscriber::set_global_default(subscriber)
        .expect("Failed to setup tracing subscriber");

    let db = Arc::new(database::DB::new(&app.db_url).await?);

    let omegastrikers_client =
        omegastrikers::OmegaApiClient::new_from_file(&app.omegastrikers_identity_file)?;

    let mut discord_bot =
        discord::Bot::new(&app.discord_bot_token, db.clone(), omegastrikers_client).await?;

    let _handler = tokio::spawn(async move { discord_bot.start().await });

    let router = init_router(routes::AppState {
        http_client: reqwest::Client::new(),
        oauth_config: OAuthConfig {
            startgg_client_id: app.startgg_oauth_client_id.clone(),
            startgg_client_secret: app.startgg_oauth_client_secret.clone(),
            startgg_redirect_uri: app.startgg_redirect_uri.clone(),
        },
    });
    let listener = TcpListener::bind(&app.listen_address).await?;

    axum::serve(listener, router.into_make_service()).await?;

    Ok(())
}
