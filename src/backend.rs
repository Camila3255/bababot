use std::{error::Error, fmt::Display, str::FromStr};

use serenity::{model::prelude::*, prelude::Context};

const PREFIX: &str = "-";
pub const BABACORD_ID: u64 = 556333985882439680;
pub const STAFF_ROLE: u64 = 564541527108616193;

/// A representation of a given bot command.
pub enum Command {
    /// Bans a user
    Ban(UserId),
    /// Mutes a user for a specified time
    Mute(UserId, Time),
    /// Gives a mod notice to the current channel
    Notice(String),
    /// Gives a message privately to the staff bot channel
    PrivateModMessage { message: String, user: String },
    /// Shows an XKCD link
    Xkcd(u32),
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
                Self::Ban(_) | Self::Mute(_, _) | Self::Notice(_) => {
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
        match args[0] {
            "ban" => {
                let Ok(user_id) = UserId::from_str(args[1]) else {
                    return Command::NotValid("Could not parse User ID".to_owned());
                };
                Command::Ban(user_id).requires_mod(ctx, message).await
            }
            "mute" => {
                let Ok(user_id) = UserId::from_str(args[1]) else {
                    return Command::NotValid("Could not parse User ID".to_owned());
                };
                let Ok(time) = Time::from_str(args[2]) else {
                    return Command::NotValid("Could not parse time string".to_owned());
                };
                Command::Mute(user_id, time)
                    .requires_mod(ctx, message)
                    .await
            }
            "xkcd" => Command::Xkcd(args[1].parse().unwrap_or(378)),
            "notice" => {
                let notice = args
                    .clone()
                    .into_iter()
                    .flat_map(|string| string.chars())
                    .collect::<String>();
                Command::Notice(notice).requires_mod(ctx, message).await
            }
            "mail" => {
                let notice = args
                    .clone()
                    .into_iter()
                    .flat_map(|string| string.chars())
                    .collect::<String>();
                let user = message.author.name.clone();
                Command::PrivateModMessage {
                    message: notice,
                    user,
                }
            }
            arg => Command::NotValid(format!("`{arg}` is not a valid command!")),
        }
    }
    pub fn execute_command(self, _ctx: Context, _message: Message) {}
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

async fn is_mod(ctx: &Context, message: &Message) -> bool {
    message
        .author
        .has_role(ctx.clone().http, BABACORD_ID, STAFF_ROLE)
        .await
        .unwrap_or(false)
}
