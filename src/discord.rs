use anyhow::Context as _;
use poise::serenity_prelude as serenity;
use serenity::{model::gateway::GatewayIntents, ShardManager};
use std::sync::Arc;
use tracing::{error, info};

pub struct Data {}

type Context<'a> = poise::Context<'a, Data, anyhow::Error>;

#[poise::command(prefix_command, slash_command)]
async fn ping(ctx: Context<'_>) -> Result<(), anyhow::Error> {
    ctx.say("Pong!").await?;
    Ok(())
}

pub struct DiscordClient {
    shard_manager: Arc<ShardManager>,
}

impl DiscordClient {
    pub async fn new(token: String, guild_id: serenity::GuildId) -> anyhow::Result<Self> {
        let intents = GatewayIntents::GUILD_MESSAGES | GatewayIntents::MESSAGE_CONTENT;

        let framework = poise::Framework::builder()
            .options(poise::FrameworkOptions {
                commands: vec![ping()],
                ..Default::default()
            })
            .setup(move |ctx, _ready, framework| {
                Box::pin(async move {
                    info!("Logged in as {}", _ready.user.name);

                    poise::builtins::register_in_guild(
                        ctx,
                        &framework.options().commands,
                        guild_id,
                    )
                    .await?;

                    Ok(Data {})
                })
            })
            .build();

        let mut client = serenity::Client::builder(&token, intents)
            .framework(framework)
            .await
            .context("Error creating client")?;

        let shard_manager = client.shard_manager.clone();

        tokio::spawn(async move {
            if let Err(why) = client.start().await {
                error!("Client error: {:?}", why);
            }
        });

        Ok(Self { shard_manager })
    }

    pub async fn shut_down(&self) {
        self.shard_manager.shutdown_all().await;
    }
}
