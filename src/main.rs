mod backend;

use backend::*;

use serenity::model::prelude::*;
use serenity::prelude::*;

const BABACORD_ID: u64 = 556333985882439680;
const STAFF_ROLE: u64 = 564541527108616193;

#[tokio::main]
async fn main() -> Result<(), SerenityError> {
    let client = Client::builder(get_secret(), intents()).event_handler(Bot);
    client.await?;
    Ok(())
}

struct Bot;

#[async_trait::async_trait]
impl EventHandler for Bot {
    async fn message(&self, ctx: Context, message: Message) {
        let command = Command::parse_from_message(&message);
        let moderator = is_mod(&ctx, &message).await;
        command.execute_command(ctx, message);
    }
}

fn intents() -> GatewayIntents {
    use GatewayIntents as GI;
    GI::AUTO_MODERATION_EXECUTION
        .union(GI::GUILD_BANS)
        .union(GI::GUILD_MESSAGES)
        .union(GI::GUILD_MESSAGE_REACTIONS)
        .union(GI::GUILD_MEMBERS)
        .union(GI::MESSAGE_CONTENT)
        .union(GI::GUILD_MESSAGE_TYPING)
}

async fn is_mod(ctx: &Context, message: &Message) -> bool {
    message
        .author
        .has_role(ctx.clone().http, BABACORD_ID, STAFF_ROLE)
        .await
        .unwrap_or(false)
}

fn get_secret() -> String {
    include_str!("secret.txt").to_owned()
}
