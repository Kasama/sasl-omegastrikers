use serenity::all::{
    ButtonStyle, Context, CreateButton, CreateCommand, CreateCommandOption, CreateEmbed,
    CreateEmbedFooter, CreateInteractionResponse, CreateInteractionResponseMessage, Interaction,
};

use crate::database;
use crate::discord::Handler;
use crate::omegastrikers::OmegaStrikersUser;

pub const REGISTER_COMMAND: &str = "register";
pub const REGISTER_OMEGA_COMMAND: &str = "register_confirm_omega_account";
pub const CONFIRM_OMEGA_ACC_PREFIX: &str = "register_confirm_omega_account_";
pub const CONFIRM_OMEGA_ACC_CANCEL: &str = "register_cancel";

const PLAYER_NAME: &str = "omegastrikers_player_name";

pub fn register() -> CreateCommand {
    CreateCommand::new("register")
        .description("Register with the bot and link an omega strikers acount")
        .add_option(
            CreateCommandOption::new(
                serenity::all::CommandOptionType::String,
                PLAYER_NAME,
                "Your player name in Omega Strikers",
            )
            .required(true),
        )
}

#[derive(Debug, PartialEq, Eq)]
enum OmegaAccountBelongsToDiscordUser {
    Yes,
    No,
    Maybe,
}

impl OmegaAccountBelongsToDiscordUser {
    fn check(os_user: &OmegaStrikersUser, discord_user: &serenity::model::user::User) -> Self {
        match os_user.platform_ids.discord {
            Some(ref os_discord_link) => {
                if os_discord_link.has_full_account {
                    if os_discord_link.discord_id == discord_user.id.to_string() {
                        Self::Yes
                    } else {
                        Self::No
                    }
                } else {
                    Self::Maybe
                }
            }
            _ => Self::Maybe,
        }
    }
}

impl Handler {
    pub async fn run_register(&self, ctx: Context, interaction: Interaction) {
        if let Interaction::Command(command) = interaction {
            match command.data.name.as_str() {
                REGISTER_COMMAND => {
                    let player_name = command
                        .data
                        .options()
                        .iter()
                        .find(|d| d.name == PLAYER_NAME)
                        .map(|e| match e.value {
                            serenity::all::ResolvedValue::String(s) => s,
                            _ => panic!("got invalid value type for parameter '{}'", PLAYER_NAME),
                        })
                        .unwrap_or_else(|| {
                            panic!("interaction missing required parameter '{}'", PLAYER_NAME)
                        });

                    let _ = match self.omega_api_client.search_user_name(player_name).await {
                        Ok(possible_users) => {
                            let mut intraction_response_message =
                            CreateInteractionResponseMessage::new().ephemeral(false);

                            let certain_user = possible_users.iter().find(|u| {
                                OmegaAccountBelongsToDiscordUser::check(u, &command.user)
                                == OmegaAccountBelongsToDiscordUser::Yes
                            });

                            // Uncomment this for testing
                            // let certain_user = possible_users.first();

                            if let Some(u) = certain_user {
                                let usr = database::user::User {
                                    username: command.user.name.clone(),
                                    discord: command.user.id.to_string(),
                                    omegastrikers_id: Some(u.player_id.clone()),
                                    startgg_id: None,
                                };

                                let _ = self.db.upsert_user(&usr).await.inspect_err(|e| {
                                    tracing::error!("failed to upsert user: {:?}: {e:?}", usr)
                                });

                                let embed = CreateEmbed::new()
                                    .url(format!(
                                        "https://stats.omegastrikers.gg/get_username/{}",
                                        u.player_id
                                    ))
                                    .title(&u.username);
                                intraction_response_message = intraction_response_message.embed(embed);

                                intraction_response_message = intraction_response_message
                                    .content(format!("Usuário linkado com {}:", u.username));
                            } else {
                                let choices = possible_users.iter().filter(|u| {
                                    OmegaAccountBelongsToDiscordUser::check(u, &command.user)
                                    != OmegaAccountBelongsToDiscordUser::No
                                });
                                let mut num_choices = 0;
                                for u in choices {
                                    num_choices += 1;
                                    let button = CreateButton::new(format!(
                                        "{}{}",
                                        CONFIRM_OMEGA_ACC_PREFIX, u.player_id
                                    ))
                                        .label(format!("Eu sou '{}'", u.username));

                                    let embed = CreateEmbed::new()
                                        .url(format!(
                                            "https://stats.omegastrikers.gg/get_username/{}",
                                            u.player_id
                                        ))
                                        .title(&u.username);

                                    intraction_response_message =
                                        intraction_response_message.button(button);
                                    intraction_response_message =
                                        intraction_response_message.add_embed(embed);
                                }
                                if num_choices > 0 {
                                    let button = CreateButton::new(CONFIRM_OMEGA_ACC_CANCEL)
                                        .label("Não sou nenhum desses")
                                        .style(ButtonStyle::Danger);
                                    intraction_response_message =
                                        intraction_response_message.button(button);

                                    let anti_fake_notice = CreateEmbed::new()
                                        .footer(CreateEmbedFooter::new("Lembre-se que associar uma conta que não é sua pode acarretar em punições e afetar sua eligibilidade para os campeonatos da SASL"));
                                    intraction_response_message =
                                        intraction_response_message.add_embed(anti_fake_notice);
                                }
                                intraction_response_message = intraction_response_message.content(
                                    if num_choices == 0 {
                                        if possible_users.len() > num_choices {
                                            "Encontrei algum usuário para essa busca, mas a conta do Omega Strikers está linkada com outro discord."
                                        } else {
                                            "Não encontrei nenhum resultado para esse usuário."
                                        }
                                    } else if num_choices == 1 {
                                        "Encontrei essa opção, mas não tenho certeza se é você: <t:1743372286:F>"
                                    } else {
                                        "Encontrei essas opções, mas não tenho certeza de qual você é:"
                                    }
                                );
                            }

                            let builder = CreateInteractionResponse::Message(intraction_response_message);
                            command.create_response(&ctx.http, builder)
                        }
                        Err(e) => command.create_response(
                            &ctx.http,
                            CreateInteractionResponse::Message(
                                CreateInteractionResponseMessage::new()
                                    .ephemeral(false)
                                    .content(format!(
                                        "Failed to get player name from omega strikers: {e:?}"
                                    )),
                            ),
                        ),
                    }
                    .await;
                }
                REGISTER_OMEGA_COMMAND => {
                    let usr = database::user::User {
                        username: command.user.name.clone(),
                        discord: command.user.id.to_string(),
                        omegastrikers_id: Some("test".to_string()),
                        startgg_id: None,
                    };

                    let _ = self.db.upsert_user(&usr).await.inspect_err(|e| {
                        tracing::error!("failed to upsert user: {:?}: {e:?}", usr)
                    });
                }
                _ => {}
            }
        } else if let Interaction::Component(component) = interaction {
            if component.data.custom_id == CONFIRM_OMEGA_ACC_CANCEL {
                let usr = database::user::User {
                    username: component.user.name.clone(),
                    discord: component.user.id.to_string(),
                    omegastrikers_id: None,
                    startgg_id: None,
                };

                let _ =
                    self.db.upsert_user(&usr).await.inspect_err(|e| {
                        tracing::error!("failed to upsert user: {:?}: {e:?}", usr)
                    });
            } else if let Some(id) = component
                .data
                .custom_id
                .strip_prefix(CONFIRM_OMEGA_ACC_PREFIX)
            {
                let usr = database::user::User {
                    username: component.user.name.clone(),
                    discord: component.user.id.to_string(),
                    omegastrikers_id: Some(id.to_string()),
                    startgg_id: None,
                };

                let _ =
                    self.db.upsert_user(&usr).await.inspect_err(|e| {
                        tracing::error!("failed to upsert user: {:?}: {e:?}", usr)
                    });

                let builder = CreateInteractionResponse::Message(
                    CreateInteractionResponseMessage::new()
                        .content("Associado")
                        .ephemeral(true),
                );

                // let builder = CreateInteractionResponseFollowup::new().content("Associado");
                let int = component.get_response(&ctx.http).await;
                tracing::info!("responses: {int:?}");

                let _ = component
                    .create_response(&ctx.http, builder)
                    // .edit_followup(&ctx.http, component.message.id, builder)
                    // .edit_response(&ctx.http, builder)
                    .await
                    .inspect_err(|e| {
                        tracing::error!("Failed to edit response after updating user: {e:?}")
                    });
            }
        }
    }
}
