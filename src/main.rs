pub mod backend;
pub mod casefile;
pub mod shard;

use backend::*;
use eyre::Result;
use serenity::{
    model::prelude::{GatewayIntents, Message},
    prelude::{Client, Context, EventHandler, SerenityError},
};
use shard::BotShard;
use std::{env, io::Result as IOResult};

#[tokio::main]
async fn main() -> Result<()> {
    create_files()?;
    let mut client = Client::builder(get_secret()?, intents())
        .event_handler(Bot)
        .await?;
    client.start().await?;
    Ok(())
}

struct Bot;

#[async_trait::async_trait]
impl EventHandler for Bot {
    async fn message(&self, ctx: Context, message: Message) {
        let shard = BotShard::new(&ctx, &message);
        // keke override: if message starts with "i'm" or "i am",
        // and user is opted in, change username
        if shard.is_kekeable().await.unwrap_or(false) {
            let _ = shard.keke_author().await;
        }
        // DM override: if message is sent to bot,
        // send message to cami
        if let MessageOrigin::PrivateChannel = shard.message_origin() {
            if let Err(e) = shard
                .message_user(
                    CAMILA,
                    format!(
                        "Incoming message from {}:\n> {}",
                        shard.author(),
                        shard.original_message().content.clone()
                    ),
                )
                .await
            {
                eprintln!("Unable to send message: {e}");
            }
        }
        if let Err(e) = shard.execute_command().await {
            eprintln!("Unable to execute command: {e}");
        }
    }
}

fn intents() -> GatewayIntents {
    use GatewayIntents as GI;
    GI::all()
}

/// Gets the bot token to log into the bot account.
/// Uses environment variables. The following are checked for having a token,
/// in the given order
/// - BABA_BOT_TOKEN
/// - BOT_TOKEN
/// - TOKEN
/// - BOT
fn get_secret() -> Result<String> {
    ["BABA_BOT_TOKEN", "BOT_TOKEN", "TOKEN", "BOT"]
        .into_iter()
        .flat_map(env::var)
        .next()
        .ok_or(SerenityError::Other("could not find a valid bot token").into())
}
/// Creates some nessecary files for the bot to function.
/// These are:
/// - `optin.txt`, a text file storing the user IDs who are opted into the bot.
/// - `blacklist.txt`, a text file storing the user IDs who are blocked from using the bot.
/// - `casefiles`, a folder containing all the various casefiles.
fn create_files() -> IOResult<()> {
    use std::fs as files;
    files::File::create("optin.txt")?;
    files::File::create("blacklist.txt")?;
    files::create_dir("casefiles")?;
    Ok(())
}

#[cfg(test)]
mod test {
    use std::str::FromStr;

    use indoc::indoc;

    use crate::{casefile::CaseFile, *};
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
    #[test]
    fn command_parse_ban() {
        let target = CommandType::Ban;
        let parsed = "-ban foo_bar".parse().unwrap();
        assert_eq!(target, parsed);
    }
    #[test]
    fn command_parse_mute() {
        let target = CommandType::Mute;
        let parsed = "-mute foo_bar reason: amogus".parse().unwrap();
        assert_eq!(target, parsed);
    }
    #[test]
    fn command_parse_pvm() {
        let target = CommandType::PrivateModMessage;
        let parsed = "-pvm general chat is breaking rule 5".parse().unwrap();
        assert_eq!(target, parsed);
    }
    #[test]
    fn command_parse_da2a() {
        let target = CommandType::DontAskToAsk;
        let parsed = "-da2a".parse().unwrap();
        assert_eq!(target, parsed);
    }
    #[test]
    fn command_parse_dont_ask_to_ask() {
        let target = CommandType::DontAskToAsk;
        let parsed = "-dontasktoask".parse().unwrap();
        assert_eq!(target, parsed);
    }
    #[test]
    fn command_parse_help() {
        let target = CommandType::Help;
        let parsed = "-help da2a".parse().unwrap();
        assert_eq!(target, parsed);
    }
    #[test]
    fn command_parse_xkcd() {
        let target = CommandType::Xkcd;
        let parsed = "-xkcd python".parse().unwrap();
        assert_eq!(target, parsed);
    }
    #[test]
    fn command_parse_notice() {
        let target = CommandType::Notice;
        let parsed = "-notice please keep in mind rule 1984".parse().unwrap();
        assert_eq!(target, parsed);
    }

    #[test]
    fn casefile_parsing_creation() {
        let file = indoc! {"
            Foo v. Bar|unresolved
            - Among us
        "}
        .parse::<CaseFile>();
        assert!(file.is_ok());
    }
    #[test]
    fn casefile_parsing_equality() {
        let file = indoc! {"
            Foo v. Bar|unresolved
            - Among us
        "}
        .parse::<CaseFile>()
        .unwrap();
        assert_eq!(
            file,
            CaseFile {
                name: "Foo v. Bar".to_owned(),
                resolved: false,
                items: vec!["Among Us".to_owned()]
            }
        )
    }
}
