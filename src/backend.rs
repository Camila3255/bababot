//! deals with parsing and preforming commands,
//! particularly with the [`Command`] enum.

use crate::shard::BotShard;
use chrono::Duration;
use eyre::Result;
use indoc::indoc;
use rand::random;
use serenity::{
    model::prelude::{Timestamp, UserId},
    Error as SerenityError,
};
use std::{
    convert::Infallible, error::Error, fmt::Display, fs as files, num::ParseIntError, str::FromStr,
    time::Duration as StdDuration,
};

/// The prefix for the bot. Messages must start with this to invoke the bot,
/// else the command is ignored.
pub const PREFIX: &str = "-";
/// The ID for the current developer of the bot.
/// Used to validate [`Command::Dev`] commands.
pub const CAMILA: u64 = 284883095981916160;

/// A representation of a given bot command.
#[derive(Debug, PartialEq, Eq)]
pub enum Command {
    /// Bans a user, with a reason
    Ban(UserId, String),
    /// Mutes a user for a specified time and reason
    Mute(UserId, Time, String),
    /// Gives a mod notice to the current channel
    Notice(String),
    /// Gives a message privately to the staff bot channel
    PrivateModMessage {
        #[doc = "The message to send"]
        message: String,
        #[doc = "The relevant user"]
        user: String,
    },
    /// Shows an XKCD link
    Xkcd(u64),
    /// Sends, literally, https://dontasktoask.com/
    DontAskToAsk,
    /// Help Command
    Help(Option<CommandType>),
    /// A suggestion for the bot
    Suggestion(String),
    /// The command wasn't valid (for one reason or another)
    NotValid(String),
    /// The message wasn't a given command
    NotACommand,
    /// A developer command
    Dev(String),
    /// A single coin flip
    CoinFlip,
    /// A randomly generated integer from 0 to [the field]
    RandomInt(u64),
    /// Opt into getting keke'd
    Optin,
    /// Opt out of get keke'd
    Optout,
    /// Sends a link to the original "get keke'd" video
    Keke,
}

impl Command {
    /// Tells a command that a moderator role is required.
    /// If the role is not present, the command is turned into [`Command::NotValid`],
    /// else the command is returned unchanged.
    pub async fn requires_mod(self, shard: BotShard<'_>) -> Self {
        if let Ok(b) = shard.user_is_mod(shard.author().id.0).await {
            match b {
                true => self,
                false => match self {
                    Self::Ban(..) | Self::Mute(..) | Self::Notice(..) => {
                        Self::NotValid("User is not a moderator!".to_owned())
                    }
                    elsewise => elsewise,
                },
            }
        } else {
            Self::NotValid("Could not determine whether the user is a mod, so I'm falling back to not allowing it.".to_owned())
        }
    }
    /// Tells a command that being the developer is required.
    /// If the developer did not issue the statement,
    /// the command is turned into [`Command::NotValid`].
    pub async fn requires_dev(self, shard: BotShard<'_>) -> Self {
        if shard.author_id().await == CAMILA {
            self
        } else {
            Self::NotValid("User is not the dev!".to_owned())
        }
    }
    /// Parses a command given a [`Context`] and a sent [`Message`].
    pub async fn parse_from_message(shard: BotShard<'_>) -> Self {
        if !shard.original_message().content.starts_with(PREFIX) {
            return Command::NotACommand;
        }
        let args = shard
            .original_message()
            .content
            .split(|chr: char| chr.is_whitespace())
            .collect::<Vec<_>>();
        if args.is_empty() {
            return Command::NotACommand;
        }
        match args[0]
            .strip_prefix(PREFIX)
            .expect("fn returns early if message starts with prefix")
            .parse::<CommandType>()
            .unwrap_or(CommandType::NotValid)
        {
            CommandType::Ban => {
                let Ok(user_id) = UserId::from_str(args[1]) else {
                    return Command::NotValid("Given user was not a valid UserID".to_owned());
                };
                let reason = vec_str_to_string(&args, Some(1));
                Command::Ban(user_id, reason).requires_mod(shard).await
            }
            CommandType::Mute => {
                let Ok(user_id) = UserId::from_str(args[1]) else {
                    return Command::NotValid("Given user was not a valid UserID".to_owned());
                };
                let Ok(time) = Time::from_str(args[2]) else {
                    return Command::NotValid("Given time was invalid!".to_owned());
                };
                Command::Mute(user_id, time, vec_str_to_string(&args, Some(3)))
                    .requires_mod(shard)
                    .await
            }
            CommandType::Notice => {
                Command::Notice(vec_str_to_string(&args, Some(1)))
                    .requires_mod(shard)
                    .await
            }
            CommandType::PrivateModMessage => Command::PrivateModMessage {
                message: vec_str_to_string(&args, Some(1)),
                user: shard.original_message().author.name.clone(),
            },
            CommandType::Xkcd => {
                Command::Xkcd(xkcd_from_string(&vec_str_to_string(&args, Some(1))))
            }
            CommandType::DontAskToAsk => Command::DontAskToAsk,
            CommandType::NotValid => Command::NotValid("I couldn't parse the command!".to_owned()),
            CommandType::NotACommand => Command::NotACommand,
            CommandType::Help => Command::Help({
                if args.len() == 1 {
                    None
                } else {
                    Some(
                        vec_str_to_string(&args, Some(1))
                            .parse()
                            .expect("Parsing a command is infallible"),
                    )
                }
            }),
            CommandType::Suggestion => Command::Suggestion(vec_str_to_string(&args, Some(1))),
            CommandType::Dev => {
                Command::Dev(vec_str_to_string(&args, Some(1)))
                    .requires_dev(shard)
                    .await
            }
            CommandType::CoinFlip => Command::CoinFlip,
            CommandType::RandomInt => {
                if let Ok(int) = vec_str_to_string(&args, Some(1)).parse::<u64>() {
                    Command::RandomInt(int)
                } else {
                    Command::NotValid(
                        "Couldn't parse an integer from the given arguments!".to_owned(),
                    )
                }
            }
            CommandType::Optin => Command::Optin,
            CommandType::Optout => Command::Optout,
            CommandType::Keke => Command::Keke,
        }
    }
    /// Executes a command.
    /// Any errors from the process are bubbled up.
    pub async fn execute_command(self, shard: BotShard<'_>) -> Result<()> {
        match self {
            Command::Ban(user, reason) => {
                let user = shard.member_request(user).await?;
                let message = format!(
                    "Successfully banned {} for the following reason: \n>{reason}",
                    user.user.name
                );
                user.ban_with_reason(shard.http_server(), 0, &reason)
                    .await?;
                shard.message_user(user.user.id.0, indoc! {"
                    You were given a ban in the __Baba is You Discord Server__ for the following reason:
                    > *[REASON]*
                    If you think was done in error, you can DM the staff for appeal. 
                    We recommend waiting at least a week for appeals!
                    Note that a long time having been passed is not usually enough for an appeal.
                    
                    There is no chance for appeal if the ban was for the following reasons:
                    ❌Being discriminatory in any form.
                    ❌Breaking discord's ToS or sharing otherwise illegal content.
                    ❌Pirating Baba is You or sharing other pirated media.
                    ❌Promoting Cryptocurrencies, misinformation, or other unwarranted advertisements.
                    
                    There are cases where appeal is guaranteed:
                    ✅If your account was compromised and banned for being so, and you have regained access to the account.
                    ✅Having pirated Baba is You, but then purchasing it legitimately.
                    ✅Being banned for being underage, but then being of a legal age to join in the user's country.
                "}.replace("[REASON]", &reason)).await?;
                shard.send_message(message).await?;
            }
            Command::Mute(user_id, time, reason) => {
                let message =
                    format!("Successfully muted user for the following reason: \n>{reason}");
                shard.mute_user(user_id, time, &reason).await?;
                shard.message_user(user_id, indoc! {"
                    You were given a mute in the __Baba is You Discord Server__ for the following reason:
                    > *[REASON]*
                    If you beleive this to be in error, contact the staff team.
                "}.replace("[REASON]", &reason)).await?;
                shard.send_message(message).await?;
            }
            Command::Notice(message) => {
                shard.send_message(format!(
                    "The following is an official announcement from the Baba is You staff team:\n> **{message}**"
                )).await?;
            }
            Command::PrivateModMessage { .. } => {
                shard.send_message("One-Time private mod messages are unimplemented. For now, you can use the modmail system.").await?;
            }
            Command::Xkcd(id) => {
                shard
                    .send_message(format!("https://xkcd.com/{id}/"))
                    .await?;
            }
            Command::DontAskToAsk => {
                shard.send_message("https://dontasktoask.com/").await?;
            }
            Command::Help(command) => {
                if let Some(command) = command {
                    shard.send_message(command.help_message()).await?;
                } else {
                    shard
                        .send_message(indoc! {"
                        Availible Commands:
                    "})
                        .await?;
                }
            }
            Command::Suggestion(suggestion) => {
                shard
                    .message_user(
                        CAMILA,
                        format!("Heads up Cami! Someone sent in a suggestion:\n> {suggestion}"),
                    )
                    .await?;
                shard.send_message("Successfully sent suggestion off to Cami!\nIf this is an emergency, I'd reccomend pinging her.").await?;
            }
            Command::NotValid(reason) => {
                shard
                    .send_message(
                        "Oops! That command was invalid for the following reason: \n> [REASON]"
                            .replace("[REASON]", &reason),
                    )
                    .await?;
            }
            Command::NotACommand => { /*intentionally do nothing*/ }
            Command::Dev(action) => match action.as_str() {
                "stop" | "halt" => {
                    let _ = shard.send_message("Shutting down...").await;
                    std::process::abort();
                }
                _ => {}
            },
            Command::CoinFlip => {
                let flip = match random::<bool>() {
                    true => "heads",
                    false => "tails",
                };
                shard
                    .send_message(format!("The result of the coin flip was... ||{flip}!||"))
                    .await?;
            }
            Command::RandomInt(bound) => {
                let int = (random::<f64>() * bound as f64) as u64;
                shard
                    .send_message(format!("Between 0 and {bound}, I choose... ||{int}!||"))
                    .await?;
            }
            Command::Optin => {
                let user = shard.author();
                let mut file = files::read_to_string("optin.txt")?
                    .lines()
                    .map(ToOwned::to_owned)
                    .collect::<Vec<_>>();
                if !file.contains(&format!("{}", user.id.0)) {
                    file.push(format!("{}", user.id.0));
                }
                files::write("optin.txt", vec_string_to_string(&file, None))
            }?,
            Command::Optout => {
                let user = shard.author();
                let mut file = files::read_to_string("optin.txt")?
                    .lines()
                    .map(ToOwned::to_owned)
                    .collect::<Vec<_>>();
                if file.contains(&format!("{}", user.id.0)) {
                    file.retain(|item| item != &format!("{}", user.id.0));
                }
                files::write("optin.txt", vec_string_to_string(&file, None))
            }?,
            Command::Keke => {
                shard.send_message(
                    "https://cdn.discordapp.com/attachments/563196186912096256/799820975666888764/SPOILER_Untitled_28_1080p.mp4"
                ).await?;
            }
        }
        Ok(())
    }
}

/// A representation of a time string (e.g. "2h30m")
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Time {
    /// Number of seconds
    pub seconds: u8,
    /// Number of minutes
    pub minutes: u8,
    /// number of hours
    pub hours: u8,
    /// number of days
    pub days: u8,
}

impl TryFrom<Time> for Timestamp {
    type Error = eyre::Report;
    fn try_from(value: Time) -> Result<Self> {
        let duration = {
            let mut duration = StdDuration::default();
            // seconds
            duration += StdDuration::new(value.seconds.into(), 0);
            // minutes
            duration += StdDuration::new((value.minutes * 60).into(), 0);
            // hours
            duration += StdDuration::new((value.hours * 60 * 60).into(), 0);
            // days
            duration += StdDuration::new((value.days * 60 * 60 * 24).into(), 0);
            Duration::from_std(duration)
        }?;
        let stamp = Timestamp::now()
            .checked_add_signed(duration)
            .ok_or_else(|| SerenityError::Other("Timestamp overflow"))?;
        Ok(stamp.into())
    }
}

impl FromStr for Time {
    type Err = TimeErr;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let allowed_chars = ['s', 'm', 'h', 'd'];
        let mut time = Time::default();
        for each in s.split_inclusive(|chr: char| allowed_chars.contains(&chr)) {
            let (time_change, duration): (String, String) =
                each.chars().partition(|x| !x.is_alphabetic());
            match time_change.clone().parse::<u8>() {
                Ok(val) => {
                    match duration.chars().next().unwrap_or('\\') {
                        's' => time.seconds = val,
                        'm' => time.minutes = val,
                        'h' => time.hours = val,
                        'd' => time.days = val,
                        '\\' => return Err(TimeErr::NoTimeSpecifier),
                        chr => return Err(TimeErr::InvalidTimeSpecifier(chr)),
                    };
                }
                Err(e) => return Err(TimeErr::ParseIntError(e)),
            }
        }
        Ok(time)
    }
}
/// Represents an error from parsing a timestamp
#[derive(Debug)]
pub enum TimeErr {
    /// There was an invalid time specifier (only valid ones are 's', 'm', 'h', and 'd')
    InvalidTimeSpecifier(char),
    /// There was an error when parsing an integer
    ParseIntError(ParseIntError),
    /// No time specifier was given
    NoTimeSpecifier,
}

impl Error for TimeErr {}

impl Display for TimeErr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TimeErr::InvalidTimeSpecifier(chr) => write!(
                f,
                "{chr} is not a valid time specifier - only 's', 'm', 'h', and 'd' are valie"
            ),
            TimeErr::ParseIntError(e) => write!(f, "parse int error: {e}"),
            TimeErr::NoTimeSpecifier => write!(f, "no time specifier was given"),
        }
    }
}

/// Represents a type of command
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum CommandType {
    /// A ban
    Ban,
    /// A mute
    Mute,
    /// An anonymous mod notice
    Notice,
    /// A private mod message
    PrivateModMessage,
    /// An XKCD link
    Xkcd,
    /// dontasktoask.com
    DontAskToAsk,
    /// Not a valid command
    NotValid,
    /// Not a command
    NotACommand,
    /// A help command
    Help,
    /// A suggestion
    Suggestion,
    /// A dev command
    Dev,
    /// Flips a coin
    CoinFlip,
    /// A random integer between 0 and a bound
    RandomInt,
    /// Opts into being keke'd
    Optin,
    /// Opts out of being keke'd
    Optout,
    /// kekes
    Keke,
}

impl CommandType {
    #[allow(dead_code)]
    /// Returns the associated (and pre-formatted) help message
    /// for a given [`Command`].
    pub fn help_message(&self) -> String {
        match self {
            CommandType::Ban => indoc! {"
                ```
                {prefix}ban [user] - Mod Only!
                ================================
                Bans a user from the server. Note that bans require, at least,
                half or more of the mod team to agree to ban someone in most cases.
                ```
            "}
            .replace("{prefix}", PREFIX),
            CommandType::Mute => indoc! {"
                ```
                {prefix}mute [user] [time] [reason] - Mod Only!
                ================================
                Mutes a user for a specified time.
                This uses discord's 'Time Out' feature,
                rather than
                ```
            "}
            .replace("{prefix}", PREFIX),
            CommandType::Notice => indoc! {"
                ```
                {prefix}notice [...message] - Mod Only!
                ================================
                Anonymously gives a broadcast to the channel.
                ```
            "}
            .replace("{prefix}", PREFIX),
            CommandType::PrivateModMessage => indoc! {"
                ```
                {prefix}pvm [...message]
                ================================
                Sends a one-time message to the mod channel.
                ```
            "}
            .replace("{prefix}", PREFIX),
            CommandType::Xkcd => indoc! {"
                ```
                {prefix}xkcd [<index:number> OR <phrase:word(s)>]
                ================================
                Sends a pre-formatted XKCD link.
                Some phrases have link mappings (e.g. 'tautology' maps to XKCD 703.)
                ```
            "}
            .replace("{prefix}", PREFIX),
            CommandType::DontAskToAsk => indoc! {"
                ```
                {prefix}da2a | {prefix}dontasktoask
                ================================
                Sends the link 'https://dontasktoask.com/', verbatim.
                ```
            "}
            .replace("{prefix}", PREFIX),
            CommandType::NotValid => indoc! {"
                ```
                iNVALID COMMAND
                ```
            "}
            .replace("{prefix}", PREFIX),
            CommandType::NotACommand => indoc! {"
                ```
                INVALID COMMAND
                ```
            "}
            .replace("{prefix}", PREFIX),
            CommandType::Help => indoc! {"
                ```
                {prefix}help <command>
                ================================
                Hey, wait a minute...
                ```
            "}
            .replace("{prefix}", PREFIX),
            CommandType::Suggestion => indoc! {"
                ```
                {prefix}suggest [phrase:word(s)]
                ================================
                Sends a suggestion to be reviewed at a later date.
                ```
            "}
            .replace("{prefix}", PREFIX),
            CommandType::Dev => indoc! {"
                ```
                {prefix}dev [command] - Dev Only!
                ================================
                Can preform a variety of developer options.
                ```
            "}
            .replace("{prefix}", PREFIX),
            CommandType::CoinFlip => indoc! {"
                ```
                {prefix}coinflip
                ================================
                50/50 chance to return Heads or Tails.
                ```
            "}
            .replace("{prefix}", PREFIX),
            CommandType::RandomInt => indoc! {"
                ```
                {prefix}randint [max:number]
                ================================
                Returns a random number between 0 and max, inclusive of both.
                ```
            "}
            .replace("{prefix}", PREFIX),
            CommandType::Optin => indoc! {"
                ```
                {prefix}optin
                ================================
                Allows you to get keke'd.
                Specifically, your name can be changed by saying 'I'm ___' or a similar phrase.
                ```
            "}
            .replace("{prefix}", PREFIX),
            CommandType::Optout => indoc! {"
                ```
                {prefix}optout
                ================================
                Opts out of getting keke'd.
                ```
            "}
            .replace("{prefix}", PREFIX),
            CommandType::Keke => indoc! {"
                ```
                {prefix}keke
                ================================
                Sends the original 'lmao get keke'd' video.
                ```
            "}
            .replace("{prefix}", PREFIX),
        }
    }
}

impl From<Command> for CommandType {
    fn from(value: Command) -> Self {
        match value {
            Command::Ban(..) => Self::Ban,
            Command::Mute(..) => Self::Mute,
            Command::Notice(_) => Self::Notice,
            Command::PrivateModMessage { .. } => Self::PrivateModMessage,
            Command::Xkcd(_) => Self::Xkcd,
            Command::DontAskToAsk => Self::DontAskToAsk,
            Command::NotValid(_) => Self::NotValid,
            Command::NotACommand => Self::NotACommand,
            Command::Help(_) => Self::Help,
            Command::Suggestion(_) => Self::Suggestion,
            Command::Dev(_) => Self::Dev,
            Command::CoinFlip => Self::CoinFlip,
            Command::RandomInt(_) => Self::RandomInt,
            Command::Optin => Self::Optin,
            Command::Optout => Self::Optout,
            Command::Keke => Self::Keke,
        }
    }
}

impl FromStr for CommandType {
    type Err = Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // remove the prefix and get the first argument
        let binding = s
            .strip_prefix('-')
            .unwrap_or(s)
            .split(|chr| matches!(chr, ' ' | '\n'))
            .collect::<Vec<_>>();
        let prefix = *binding.first().unwrap_or(&"");
        Ok(match prefix.to_lowercase().as_str() {
            "ban" => Self::Ban,
            "mute" => Self::Mute,
            "notice" => Self::Notice,
            "private" | "pvm" => Self::PrivateModMessage,
            "xkcd" => Self::Xkcd,
            "dontasktoask" | "da2a" => Self::DontAskToAsk,
            "help" => Self::Help,
            "suggest" => Self::Suggestion,
            "dev" => Self::Dev,
            "coinflip" | "flip" => Self::CoinFlip,
            "randint" | "rand" => Self::RandomInt,
            "optin" => Self::Optin,
            "optout" => Self::Optout,
            "keke" => Self::Keke,
            _ => Self::NotValid,
        })
    }
}

/// Represents the origin of a message (either private or public)
pub enum MessageOrigin {
    /// A public channel (inside a server)
    PublicChannel,
    /// A private channel (inside a DM)
    PrivateChannel,
}

/// Gets an xkcd from a string.
/// if the string isn't able to be parsed as a number,
/// some special keywords link to certain comics.
pub fn xkcd_from_string(string: &str) -> u64 {
    if let Ok(val) = string.parse() {
        val
    } else {
        match string.to_lowercase().as_str() {
            "tautology" | "tautological" | "honor society" => 703,
            "python" | "import antigravity" | "antigravity" => 353,
            "haskell" | "side effects" => 1312,
            "trolley problem" => 1455,
            "linux" | "OS" => 272,
            _ => 404,
        }
    }
}
/// Takes a slice of &[`str`] and an optional index, and returns a [`String`]
/// of the concatenated items.
/// If an index is provided, only the items from that index and onward
/// are concatenated.
pub fn vec_str_to_string(vector: &[&str], idx: Option<usize>) -> String {
    let vector = vector
        .iter()
        .copied()
        .map(|x| x.to_owned())
        .collect::<Vec<_>>();
    if let Some(index) = idx {
        let slice = &vector[index..];
        slice.join(" ")
    } else {
        vector.join(" ")
    }
}

/// Takes a slice of &[`String`] and an optional index, and returns a [`String`]
/// of the concatenated items.
/// If an index is provided, only the items from that index and onward
/// are concatenated.
pub fn vec_string_to_string(vector: &[String], idx: Option<usize>) -> String {
    let vector = vector.to_vec();
    if let Some(index) = idx {
        let slice = &vector[index..];
        slice.join(" ")
    } else {
        vector.join(" ")
    }
}
