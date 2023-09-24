use std::{convert::Infallible, error::Error, fmt::Display, str::FromStr};

use serenity::{model::prelude::*, prelude::*, client::bridge::gateway::ShardRunnerMessage};
use tungstenite::protocol::Message as TungstenMessage;
use eyre::Result;

use indoc::indoc;

const PREFIX: &str = "-";
pub const BABACORD_ID: u64 = 556333985882439680;
pub const STAFF_ROLE: u64 = 564541527108616193;

/// A representation of a given bot command.
pub enum Command {
    /// Bans a user
    Ban(UserId),
    /// Mutes a user for a specified time and reason
    Mute(UserId, Time, String),
    /// Gives a mod notice to the current channel
    Notice(String),
    /// Gives a message privately to the staff bot channel
    PrivateModMessage { message: String, user: String },
    /// Shows an XKCD link
    Xkcd(u32),
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
}

impl Command {
    /// Tells a command that a moderator role is required.
    /// If the role is not present, the command is turned into [`Command::NotValid`],
    /// else the command is returned unchanged.
    pub async fn requires_mod(self, ctx: &Context, message: &Message) -> Self {
        if is_mod(ctx, message).await {
            self
        } else {
            match self {
                Self::Ban(_) | Self::Mute(..) | Self::Notice(_) => {
                    Self::NotValid("User is not a registered moderator".to_owned())
                }
                this => this,
            }
        }
    }
    pub async fn parse_from_message(ctx: &Context, message: &Message) -> Self {
        if !message.content.starts_with(PREFIX) {
            return Command::NotACommand;
        }
        let args = message
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
            .expect("CommandType::from_str is infallible")
        {
            CommandType::Ban => {
                let Ok(user_id) = UserId::from_str(args[1]) else {
                    return Command::NotValid("Given user was not a valid UserID".to_owned());
                };
                Command::Ban(user_id).requires_mod(ctx, message).await
            }
            CommandType::Mute => {
                let Ok(user_id) = UserId::from_str(args[1]) else {
                    return Command::NotValid("Given user was not a valid UserID".to_owned());
                };
                let Ok(time) = Time::from_str(args[2]) else {
                    return Command::NotValid("Given time was invalid!".to_owned());
                };
                Command::Mute(user_id, time, vec_string_to_string(&args, Some(3)))
                    .requires_mod(ctx, message)
                    .await
            }
            CommandType::Notice => {
                Command::Notice(vec_string_to_string(&args, Some(1)))
                    .requires_mod(ctx, message)
                    .await
            }
            CommandType::PrivateModMessage => Command::PrivateModMessage {
                message: vec_string_to_string(&args, Some(1)),
                user: message.author.name.clone(),
            },
            CommandType::Xkcd => {
                Command::Xkcd(xkcd_from_string(&vec_string_to_string(&args, Some(1))))
            }
            CommandType::DontAskToAsk => Command::DontAskToAsk,
            CommandType::NotValid => Command::NotValid("Command was not valid!".to_owned()),
            CommandType::NotACommand => Command::NotACommand,
            CommandType::Help => Command::Help(CommandType::from_str(args[1]).ok()),
            CommandType::Suggestion => Command::Suggestion(vec_string_to_string(&args, Some(1))),
        }
    }
    pub fn execute_command(self, _shard: BotShard<'_>) {
        
    }
}

/// A representation of a time string (e.g. "2h30m")
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Time {
    pub seconds: u8,
    pub minutes: u8,
    pub hours: u8,
    pub days: u8,
}

impl FromStr for Time {
    type Err = TimeErr;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let allowed_chars = ['s', 'm', 'h', 'd'];
        let mut time = Time::default();
        for each in s.split_inclusive(|chr: char| allowed_chars.contains(&chr)) {
            let (time_change, duration): (String, String) =
                each.chars().partition(|x| !x.is_alphabetic());
            if let Ok(val) = time_change.clone().parse::<u8>() {
                match duration.chars().next().unwrap_or('\\') {
                    's' => time.seconds = val,
                    'm' => time.minutes = val,
                    'h' => time.hours = val,
                    'd' => time.days = val,
                    '\\' => return Err(TimeErr::NoTimeSpecifier),
                    chr => return Err(TimeErr::InvalidTimeSpecifier(chr)),
                };
            } else {
                return Err(TimeErr::ParseIntError(time_change));
            }
        }
        Ok(time)
    }
}
#[derive(Debug)]
pub enum TimeErr {
    InvalidTimeSpecifier(char),
    ParseIntError(String),
    NoTimeSpecifier,
}

impl Error for TimeErr {}

impl Display for TimeErr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum CommandType {
    Ban,
    Mute,
    Notice,
    PrivateModMessage,
    Xkcd,
    DontAskToAsk,
    NotValid,
    NotACommand,
    Help,
    Suggestion
}

impl CommandType {
    #[allow(dead_code)]
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
            CommandType::DontAskToAsk => todo!(),
            CommandType::NotValid => todo!(),
            CommandType::NotACommand => todo!(),
            CommandType::Help => todo!(),
            CommandType::Suggestion => todo!(),
        }
    }
}

impl From<Command> for CommandType {
    fn from(value: Command) -> Self {
        match value {
            Command::Ban(_) => Self::Ban,
            Command::Mute(..) => Self::Mute,
            Command::Notice(_) => Self::Notice,
            Command::PrivateModMessage { .. } => Self::PrivateModMessage,
            Command::Xkcd(_) => Self::Xkcd,
            Command::DontAskToAsk => Self::DontAskToAsk,
            Command::NotValid(_) => Self::NotValid,
            Command::NotACommand => Self::NotACommand,
            Command::Help(_) => Self::Help,
            Command::Suggestion(_) => Self::Suggestion,
        }
    }
}

impl FromStr for CommandType {
    type Err = Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s.to_lowercase().as_str() {
            "ban" => Self::Ban,
            "mute" => Self::Mute,
            "notice" => Self::Notice,
            "private" | "pvm" => Self::PrivateModMessage,
            "xkcd" => Self::Xkcd,
            "dontasktoask" | "da2a" => Self::DontAskToAsk,
            "help" => Self::Help,
            _ => Self::NotValid,
        })
    }
}

/// Represents a shard of a bot doing calculations for a single message.
/// Has some helper methods for sending messages and interacting
/// with the inner HTTP server.
#[derive(Clone, Copy)]
pub struct BotShard<'a> {
    ctx: &'a Context,
    message: &'a Message
}

impl<'a> BotShard<'a> {
    pub fn new(ctx: &'a Context, message: &'a Message) -> Self {
        Self { ctx, message }
    }
    pub async fn command(&self) -> Command {
        Command::parse_from_message(self.ctx, self.message).await
    }
    pub async fn execute_command(&self) {
        self.command().await.execute_command(*self)
    }
    pub fn send_message(&self, message: String) -> Result<()> {
        Ok(self.ctx.shard.send_to_shard(
            ShardRunnerMessage::Message(TungstenMessage::Text(message))
        )?)
    }
}

async fn is_mod(ctx: &Context, message: &Message) -> bool {
    message
        .author
        .has_role(ctx.clone().http, BABACORD_ID, STAFF_ROLE)
        .await
        .unwrap_or(false)
}

pub fn xkcd_from_string(string: &str) -> u32 {
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

fn vec_string_to_string(vector: &[&str], idx: Option<usize>) -> String {
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
