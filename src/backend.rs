use chrono::Duration;
use eyre::Result;
use indoc::indoc;
use serenity::{http::Http, model::prelude::*, prelude::*, Result as SereneResult};
use std::{
    convert::Infallible, error::Error, fmt::Display, str::FromStr, time::Duration as StdDuration,
};
use rand::random;

const PREFIX: &str = "-";
pub const BABACORD_ID: u64 = 1095892457771782277;
pub const STAFF_ROLE: u64 = 1095892509139402782;
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
    /// A developer command
    Dev(String),
    /// A single coin flip
    CoinFlip,
    /// A randomly generated integer from 0 to [the field]
    RandomInt(u32)
}

impl Command {
    /// Tells a command that a moderator role is required.
    /// If the role is not present, the command is turned into [`Command::NotValid`],
    /// else the command is returned unchanged.
    pub async fn requires_mod(self, shard: BotShard<'_>) -> Self {
        if shard.user_is_mod(shard.author().id.0).await {
            self
        } else {
            match self {
                Self::Ban(..) | Self::Mute(..) | Self::Notice(_) => {
                    Self::NotValid("User is not a registered moderator".to_owned())
                }
                this => this,
            }
        }
    }
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
                let reason = vec_string_to_string(&args, Some(1));
                Command::Ban(user_id, reason).requires_mod(shard).await
            }
            CommandType::Mute => {
                let Ok(user_id) = UserId::from_str(args[1]) else {
                    return Command::NotValid("Given user was not a valid UserID".to_owned());
                };
                let Ok(time) = Time::from_str(args[2]) else {
                    return Command::NotValid("Given time was invalid!".to_owned());
                };
                Command::Mute(user_id, time, vec_string_to_string(&args, Some(3)))
                    .requires_mod(shard)
                    .await
            }
            CommandType::Notice => {
                Command::Notice(vec_string_to_string(&args, Some(1)))
                    .requires_mod(shard)
                    .await
            }
            CommandType::PrivateModMessage => Command::PrivateModMessage {
                message: vec_string_to_string(&args, Some(1)),
                user: shard.original_message().author.name.clone(),
            },
            CommandType::Xkcd => {
                Command::Xkcd(xkcd_from_string(&vec_string_to_string(&args, Some(1))))
            }
            CommandType::DontAskToAsk => Command::DontAskToAsk,
            CommandType::NotValid => Command::NotValid("I couldn't parse the command!".to_owned()),
            CommandType::NotACommand => Command::NotACommand,
            CommandType::Help => Command::Help({
                if args.len() == 1 {
                    None
                } else {
                    Some(
                        vec_string_to_string(&args, Some(1))
                            .parse()
                            .expect("Parsing a command is infallible"),
                    )
                }
            }),
            CommandType::Suggestion => Command::Suggestion(vec_string_to_string(&args, Some(1))),
            CommandType::Dev => {
                Command::Dev(vec_string_to_string(&args, Some(1)))
                    .requires_dev(shard)
                    .await
            }
            CommandType::CoinFlip => Command::CoinFlip,
            CommandType::RandomInt => {
                if let Ok(int) = vec_string_to_string(&args, Some(1)).parse::<u32>() {
                    Command::RandomInt(int)
                } else {
                    Command::NotValid("Couldn't parse an integer from the given arguments!".to_owned())
                }
            },
        }
    }
    /// Executes a command.
    /// Returns a string to send, if needed.
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
                    shard.send_message(indoc!{"
                        Availible Commands:
                    "}).await?;
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
                shard.send_message(format!("The result of the coin flip was... ||{flip}!||")).await?;
            },
            Command::RandomInt(bound) => {
                let int = (random::<f64>() * bound as f64) as u32;
                shard.send_message(format!("Between 0 and {bound}, I choose... ||{int}!||")).await?;
            },
        }
        Ok(())
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

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
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
    Suggestion,
    Dev,
    CoinFlip,
    RandomInt
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
            CommandType::RandomInt =>  indoc! {"
                ```
                {prefix}randint [max:number]
                ================================
                Returns a random number between 0 and max, inclusive of both.
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
    message: &'a Message,
}

#[allow(dead_code)]
impl<'a> BotShard<'a> {
    /// Creates a new [`BotShard`] given a [`Context`] and sent [`Message`].
    /// Sent messages (via `BotShard::send_message()`) are sent to
    /// the same channel as the given [`Message`].
    pub fn new(ctx: &'a Context, message: &'a Message) -> Self {
        Self { ctx, message }
    }
    /// Parses a command from the content of the given [`Message`].
    pub async fn command(&self) -> Command {
        Command::parse_from_message(*self).await
    }
    /// Executes the command from the given content of the internal [`Message`].
    pub async fn execute_command(&self) -> Result<()> {
        self.command().await.execute_command(*self).await
    }
    /// Sends a message to the same channel the given [`Message`] was sent to.
    /// Returns a [`Message`] representing the sent message.
    pub async fn send_message(&self, message: impl AsRef<str>) -> SereneResult<Message> {
        let channel_id = self.original_message().channel_id.0;
        self.send_message_to(message, channel_id).await
    }
    /// Sends a message to a given channel based on an ID.
    /// Returns the [`Message`] representing the sent message.
    pub async fn send_message_to(
        &self,
        message: impl AsRef<str>,
        channel_id: impl Into<u64>,
    ) -> SereneResult<Message> {
        let channel = self.http_server().get_channel(channel_id.into()).await?;
        if let Some(channel) = channel.clone().guild() {
            channel.say(self.http_server(), message.as_ref()).await
        } else if let Some(channel) = channel.clone().private() {
            channel.say(self.http_server(), message.as_ref()).await
        } else if channel.category().is_some() {
            return Err(SerenityError::Other("Got a category for some reason"));
        } else {
            return Err(SerenityError::Other("Not a channel"));
        }
    }
    /// Gets the author of the sent message.
    /// Useful for checking certain conditions, such as if they're a moderator.
    pub fn author(&self) -> User {
        self.message.author.clone()
    }
    /// Attempts to request a [`Member`] from the guild.
    pub async fn member_request(&self, user_id: impl Into<u64>) -> SereneResult<Member> {
        self.http_server()
            .get_member(BABACORD_ID, user_id.into())
            .await
    }
    /// Attempts to request a [`User`] from the http server.
    pub async fn user_request(&self, user_id: impl Into<u64>) -> SereneResult<User> {
        self.http_server().get_user(user_id.into()).await
    }
    /// Attempts to request a [`Channel`] from the guild.
    pub async fn channel_request(&self, channel_id: impl Into<u64>) -> SereneResult<Channel> {
        self.http_server().get_channel(channel_id.into()).await
    }
    /// Attempts to request a [`PartialGuild`] (server) from the http server.
    pub async fn server_request(&self, server_id: impl Into<u64>) -> SereneResult<PartialGuild> {
        self.http_server().get_guild(server_id.into()).await
    }
    /// A reference to the internal [`Http`] server.
    pub fn http_server(&self) -> &Http {
        &self.ctx.http
    }
    /// A reference to the given [`Context`].
    pub fn context(&self) -> &Context {
        self.ctx
    }
    /// A reference to the original [`Message`].
    pub fn original_message(&self) -> &Message {
        self.message
    }
    /// Returns whether or not a user is blacklisted.
    /// Propogated any errors associated with IO.
    pub fn user_is_blacklisted(&self, user_id: impl Into<u64>) -> Result<bool> {
        let blacklist_file = std::fs::read_to_string("src\\blacklist.txt")?;
        let blacklisted_ids = blacklist_file
            .lines()
            .map(|line| line.parse::<u64>())
            .collect::<Result<Vec<u64>, _>>()?;
        let user = user_id.into();
        for id in blacklisted_ids {
            if user == id {
                return Ok(true);
            }
        }
        Ok(false)
    }
    /// Blacklists a user.
    /// Propogates any errors associated with IO, or any [`serenity::Error`]s.
    pub async fn blacklist_user(&self, user_id: impl Into<u64>) -> Result<()> {
        let user = self.user_request(user_id.into()).await?;
        let blacklist_file = std::fs::read_to_string("src\\blacklist.txt")?;
        let mut blacklist = blacklist_file
            .lines()
            .map(|string| string.to_owned())
            .collect::<Vec<_>>();
        blacklist.push(format!("{}", user.id.0));
        let new_blacklist = blacklist.join("\n");
        std::fs::write("src\\blacklist.txt", new_blacklist)?;
        Ok(())
    }
    /// Bans a user with a reason.
    /// Reasons have a limit of 512 [`char`]s.
    pub async fn ban_user(
        &self,
        user_id: impl Into<u64>,
        reason: impl AsRef<str>,
    ) -> SereneResult<()> {
        self.member_request(user_id)
            .await?
            .ban_with_reason(self.http_server(), 0_u8, reason)
            .await
    }
    /// Mutes a user for a specified [`Time`].
    /// Returns any bubbled-up errors, or
    /// a [`Message`]
    pub async fn mute_user(
        &self,
        user_id: impl Into<u64>,
        time: Time,
        reason: impl AsRef<str>,
    ) -> Result<Message> {
        let time = time.try_into()?;
        self.member_request(user_id)
            .await?
            .disable_communication_until_datetime(self.http_server(), time)
            .await?;
        Ok(self.send_message(reason).await?)
    }
    /// Sends a message to a user.
    /// If successful, returns the associated [`Message`].
    /// Bubbles up errors.
    pub async fn message_user(
        &self,
        user_id: impl Into<u64>,
        message: impl AsRef<str>,
    ) -> SereneResult<Message> {
        self.member_request(user_id)
            .await?
            .user
            .create_dm_channel(self.http_server())
            .await?
            .say(self.http_server(), message.as_ref())
            .await
    }
    /// Returns whether a requested user is a mod.
    /// Unlike other functions, errors fallback to returning `false`.
    /// The dev always is considered a moderator.
    pub async fn user_is_mod(&self, user_id: impl Into<u64>) -> bool {
        match self.user_request(user_id).await {
            Ok(user) => match user
                .has_role(self.http_server(), BABACORD_ID, STAFF_ROLE)
                .await
            {
                Ok(b) => b || (user.id.0 == CAMILA),
                Err(_) => false,
            },
            Err(_) => false,
        }
    }
    pub async fn author_id(&self) -> u64 {
        self.author().id.0
    }
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
