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
    )]
    db_pool: PgPool,
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

    let router = init_router(
        routes::AppState::builder(
            OAuthConfig {
                startgg_client_id: startgg_oauth_client_id.to_string(),
                startgg_client_secret: startgg_oauth_client_secret.to_string(),
                startgg_redirect_uri: startgg_redirect_uri.to_string(),
            },
            db,
        )
        .build(),
    );

    Ok(router)
}
