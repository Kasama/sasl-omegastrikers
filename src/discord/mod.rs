use std::sync::Arc;

use serenity::all::{
    CreateInteractionResponse, CreateInteractionResponseMessage, EventHandler, Interaction, Ready,
};
use serenity::async_trait;
use serenity::model::channel::Message;
use serenity::prelude::*;
use tracing::{debug, info};

mod command;

use crate::database;
use crate::omegastrikers::OmegaApiClient;

struct Handler {
    db: Arc<database::DB>,
    omega_api_client: OmegaApiClient,
}

pub struct Bot {
    client: Client,
    db: Arc<database::DB>,
}

#[async_trait]
impl EventHandler for Handler {
    async fn message(&self, ctx: Context, msg: Message) {
        if let "!ping" = msg.content.as_str() {
            if let Err(e) = msg.channel_id.say(&ctx.http, "Pong").await {
                println!("Failed to send message: {e:?}")
            }
        }
    }

    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        if let Interaction::Command(ref command) = interaction {
            if command
                .data
                .name
                .starts_with(command::register::REGISTER_COMMAND)
            {
                self.run_register(ctx, interaction).await;
            } else {
                let builder = CreateInteractionResponse::Message(
                    CreateInteractionResponseMessage::new()
                        .ephemeral(true)
                        .content("not implemented"),
                );
                let _ = command.create_response(&ctx.http, builder).await;
            }
        } else if let Interaction::Component(ref component) = interaction {
            if component
                .data
                .custom_id
                .starts_with(command::register::REGISTER_COMMAND)
            {
                self.run_register(ctx, interaction).await;
            }
        }
    }

    async fn ready(&self, ctx: Context, ready: Ready) {
        info!("discord bot is ready: {}", ready.user.name);

        for guild in ready.guilds {
            let _ = guild
                .id
                .set_commands(&ctx.http, command::register_all())
                .await
                .inspect_err(|e| {
                    tracing::error!("failed to setup commands in guild {}: {:?}", guild.id, e)
                });
        }
    }
}

impl Bot {
    pub async fn new(
        token: &str,
        db: Arc<database::DB>,
        omega_api_client: OmegaApiClient,
    ) -> anyhow::Result<Self> {
        let intents = GatewayIntents::GUILD_MESSAGES
            | GatewayIntents::DIRECT_MESSAGES
            | GatewayIntents::MESSAGE_CONTENT;

        let client = Client::builder(token, intents)
            .event_handler(Handler {
                db: db.clone(),
                omega_api_client,
            })
            .await?;

        Ok(Self { client, db })
    }

    pub async fn start(&mut self) -> anyhow::Result<()> {
        debug!("starting discord bot");
        self.client.start().await?;
        Ok(())
    }
}
