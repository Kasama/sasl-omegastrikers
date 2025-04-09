use std::sync::Arc;

use axum::Router;
use sqlx::PgPool;

use self::database::DB;
use self::routes::init_router;
use self::startgg::oauth::OAuthConfig;

mod database;
mod discord;
mod omegastrikers;
mod routes;
mod startgg;

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

#[shuttle_runtime::main]
async fn shuttle_main(
    #[shuttle_shared_db::Postgres(
        local_uri = "postgres://postgres:postgres@localhost:5432/omegastrikers-sasl"
    )] db_pool: PgPool,
    #[shuttle_runtime::Secrets] secrets: shuttle_runtime::SecretStore,
) -> shuttle_axum::ShuttleAxum {

    let discord_bot_token = secrets
        .get("discord_bot_token")
        .expect("Failed to load discord_bot_token");
    let omegastrikers_identity_file = secrets
        .get("omegastrikers_identity_file")
        .unwrap_or(IDENTITY_FILE.to_string());
    let startgg_oauth_client_id = secrets
        .get("startgg_oauth_client_id")
        .expect("Failed to load startgg_oauth_client_id");
    let startgg_oauth_client_secret = secrets
        .get("startgg_oauth_client_secret")
        .expect("Failed to load startgg_oauth_client_secret");
    let startgg_redirect_uri = secrets
        .get("startgg_redirect_uri")
        .expect("Failed to load startgg_redirect_uri");

    let db = Arc::new(database::DB::from_pool(db_pool).await?);

    let router = common_main(
        db,
        &omegastrikers_identity_file,
        &discord_bot_token,
        &startgg_oauth_client_id,
        &startgg_oauth_client_secret,
        &startgg_redirect_uri,
    )
    .await?;
    Ok(shuttle_axum::AxumService(router))
}

// #[cfg(debug_assertions)]
// #[tokio::main]
// async fn main() -> anyhow::Result<()> {
//     use std::sync::Arc;

//     use clap::Parser;
//     use dotenvy::dotenv;
//     use tokio::net::TcpListener;
//     use tracing_bunyan_formatter::BunyanFormattingLayer;
//     use tracing_subscriber::layer::SubscriberExt;
//     use tracing_subscriber::EnvFilter;

//     _ = dotenv(); // ignore errors if there is no .env file

//     let app = Arc::new(App::parse());

//     let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
//     let formatting_layer = BunyanFormattingLayer::new(app.app_name.clone(), std::io::stdout);

//     let subscriber = tracing_subscriber::Registry::default()
//         .with(env_filter)
//         .with(tracing_bunyan_formatter::JsonStorageLayer)
//         .with(formatting_layer);

//     tracing::subscriber::set_global_default(subscriber)
//         .expect("Failed to setup tracing subscriber");

//     let db = Arc::new(database::DB::new(&app.db_url).await?);

//     let router = common_main(
//         db,
//         &app.omegastrikers_identity_file,
//         &app.discord_bot_token,
//         &app.startgg_oauth_client_id,
//         &app.startgg_oauth_client_secret,
//         &app.startgg_redirect_uri,
//     )
//     .await?;
//     let listener = TcpListener::bind(&app.listen_address).await?;

//     axum::serve(listener, router.into_make_service()).await?;

//     Ok(())
// }

async fn common_main(
    db: Arc<DB>,
    omegastrikers_identity_file: &str,
    discord_bot_token: &str,
    startgg_oauth_client_id: &str,
    startgg_oauth_client_secret: &str,
    startgg_redirect_uri: &str,
) -> anyhow::Result<Router> {
    // let omegastrikers_client =
    //     omegastrikers::OmegaApiClient::new_from_file(omegastrikers_identity_file)?;

    // let mut discord_bot =
    //     discord::Bot::new(discord_bot_token, db.clone(), omegastrikers_client).await?;

    // let _handler = tokio::spawn(async move { discord_bot.start().await });

    let router = init_router(routes::AppState {
        http_client: reqwest::Client::new(),
        oauth_config: OAuthConfig {
            startgg_client_id: startgg_oauth_client_id.to_string(),
            startgg_client_secret: startgg_oauth_client_secret.to_string(),
            startgg_redirect_uri: startgg_redirect_uri.to_string(),
        },
    });

    Ok(router)
}
