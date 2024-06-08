mod extensions;
mod interaction;
mod poistavahti;
mod vahti;

use std::sync::Arc;

use async_trait::async_trait;
use serenity::gateway::ShardManager;
use serenity::model::prelude::*;
use serenity::prelude::*;

use crate::command::Command;
use crate::database::Database;
use crate::error::Error;

pub const NAME: &str = "discord";

struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        interaction::handle_interaction(ctx, interaction).await;
    }

    async fn ready(&self, ctx: Context, ready: Ready) {
        info!("Connected as {}", ready.user.name);
        let _ = serenity::model::application::Command::set_global_commands(
            &ctx.http,
            vec![vahti::register(), poistavahti::register()],
        )
        .await;
    }
    async fn resume(&self, _: Context, _: ResumedEvent) {
        info!("Resumed");
    }
}

pub struct Discord {
    pub client: Client,
}

pub struct Manager {
    shard_manager: Arc<ShardManager>,
}

impl Discord {
    pub async fn init(db: &Database) -> Result<Self, Error> {
        let token =
            std::env::var("DISCORD_TOKEN").expect("Expected DISCORD_TOKEN in the environment");

        let application_id: u64 = std::env::var("APPLICATION_ID")
            .expect("Expected APPLICATION_ID in the environment")
            .parse()
            .expect("Invalid APPLICATION_ID");

        let client = Client::builder(&token, GatewayIntents::non_privileged())
            .application_id(application_id.into())
            .event_handler(Handler)
            .await?;

        let mut data = client.data.write().await;
        data.insert::<Database>(db.to_owned());
        drop(data);

        Ok(Self { client })
    }
}

#[async_trait]
impl super::Manager for Manager {
    async fn shutdown(&self) {
        info!("Discord destroy");
        self.shard_manager.shutdown_all().await;
        info!("Discord destroy done");
    }
}

#[async_trait]
impl Command for Discord {
    async fn start(&mut self) -> Result<(), Error> {
        Ok(self.client.start().await?)
    }

    fn manager(&self) -> Box<dyn super::Manager + Send + Sync> {
        Box::new(Manager {
            shard_manager: self.client.shard_manager.clone(),
        })
    }
}
