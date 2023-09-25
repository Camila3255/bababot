mod backend;

use backend::*;
use log::{error, info};
use serenity::{model::prelude::*, prelude::*};

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
        let shard = BotShard::new(&ctx, &message);
        match shard.execute_command().await {
            Ok(message) => match shard.send_message(message.clone()).await {
                Ok(_) => info!("Successfuly ran command: {message}"),
                Err(e) => error!("Unable to send message: {e}"),
            },
            Err(e) => error!("Unable to execute command: {e}"),
        }
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

fn get_secret() -> String {
    include_str!("secret.txt").to_owned()
}

#[cfg(test)]
mod test {
    use std::str::FromStr;

    use crate::*;
    #[test]
    fn time_parse_seconds() {
        let target = Time {
            seconds: 7,
            minutes: 0,
            hours: 0,
            days: 0,
        };
        let parsed = Time::from_str("7s");
        let parsed = parsed.unwrap();
        assert_eq!(target, parsed);
    }
    #[test]
    fn time_parse_minutes() {
        let target = Time {
            seconds: 0,
            minutes: 34,
            hours: 0,
            days: 0,
        };
        let parsed = Time::from_str("34m");
        let parsed = parsed.unwrap();
        assert_eq!(target, parsed);
    }

    #[test]
    fn time_parse_hours() {
        let target = Time {
            seconds: 0,
            minutes: 0,
            hours: 9,
            days: 0,
        };
        let parsed = Time::from_str("9h");
        let parsed = parsed.unwrap();
        assert_eq!(target, parsed);
    }
    #[test]
    fn time_parse_days() {
        let target = Time {
            seconds: 0,
            minutes: 0,
            hours: 0,
            days: 3,
        };
        let parsed = Time::from_str("3d");
        let parsed = parsed.unwrap();
        assert_eq!(target, parsed);
    }
    #[test]
    fn time_parse_complex() {
        let target = Time {
            seconds: 0,
            minutes: 30,
            hours: 2,
            days: 0,
        };
        let parsed = Time::from_str("2h30m");
        let parsed = parsed.unwrap();
        assert_eq!(target, parsed);
    }
}
