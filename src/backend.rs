use std::str::FromStr;

use serenity::model::prelude::*;

const PREFIX: &'static str = "-";

/// A representation of a given bot command.
pub enum Command {
    /// Bans a user
    Ban(UserId),
    /// Mutes a user for a specified time
    Mute(UserId, Time),
    /// Gives a mod notice to the current channel
    Notice(String),
    /// Gives a message privately to the staff bot channel
    PrivateModMessage(String),
    /// Shows an XKCD link
    XKCD(u32),
    /// The command wasn't valid
    NotValid(&'static str),
    /// The message wasn't a given command
    NotACommand
}

impl Command {
    pub fn requires_mod(&self) -> bool {
        match self {
            Command::Ban(_) | Command::Mute(_, _) | Command::Notice(_) => true,
            _ => false
        }
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Time {
    pub seconds: u8,
    pub minutes: u8,
    pub hours: u8,
    pub days: u8
}

impl FromStr for Time {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let allowed_chars = vec!['s', 'm', 'h', 'd'];
        let mut time = Time::default();
        for each in s.split_inclusive(|chr: char| allowed_chars.contains(&chr)) {
            let (time_change, duration): (Vec<char>, Vec<char>) = each.chars().partition(|x| x.is_alphabetic());
            if let Ok(val) = time_change.into_iter().collect::<String>().parse::<u8>() {
                match duration[0] {
                    's' => time.seconds = val,
                    'm' => time.minutes = val,
                    'h' => time.hours = val,
                    'd' => time.days = val,
                    _ => return Err(())
                };
            }
        }
        Ok(time)
    }
}

pub fn parse_command(message: Message) -> Command {
    if !message.content.starts_with(PREFIX) {
        return Command::NotACommand;
    }
    let args = message.content.split(|chr: char| chr.is_whitespace()).collect::<Vec<_>>();
    if args.is_empty() {
        return Command::NotValid("");
    }
    match args[0] {
        _ => 
    }

    Command::NotValid("Could not parse command!")
}