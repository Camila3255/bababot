use crate::backend::{Command, MessageOrigin, Time, PREFIX};
use eyre::Result;
use serenity::{
    client::{Cache, Context},
    http::Http,
    model::{
        channel::{Channel, Message},
        guild::{Member, PartialGuild},
        user::User,
        Permissions,
    },
    Error as SereneError, Result as SereneResult,
};
use std::fs as files;

/// Represents a shard of a bot doing calculations for a single message.
/// Has some helper methods for sending messages and interacting
/// with the inner HTTP server.
#[derive(Clone, Copy)]
pub struct BotShard<'a> {
    ctx: &'a Context,
    message: &'a Message,
}

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
            return Err(SereneError::Other("Got a category for some reason"));
        } else {
            return Err(SereneError::Other("Not a channel"));
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
            .get_member(self.guild_id()?, user_id.into())
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
    pub fn cache(&self) -> &Cache {
        &self.context().cache
    }
    /// Returns whether a requested user is a mod.
    /// Unlike other functions, errors fallback to returning `false`.
    /// The dev always is considered a moderator.
    pub async fn user_is_mod(&self, user_id: impl Into<u64>) -> Result<bool> {
        Ok(self
            .member_request(user_id)
            .await?
            .permissions(self.cache())?
            .contains(Permissions::BAN_MEMBERS))
    }
    pub async fn author_id(&self) -> u64 {
        self.author().id.0
    }
    /// Checks if a user is opted in AND the message is kekeable:
    /// starts with "i'm" or "i am"
    pub async fn is_kekeable(&self) -> Result<bool> {
        let opted_ins = files::read_to_string("optin.txt")?
            .lines()
            .map(|x| x.parse())
            .collect::<Result<Vec<u64>, _>>();
        let opted_ins = match opted_ins {
            Ok(vals) => vals,
            Err(_) => return Ok(false),
        };
        Ok(opted_ins.contains(&self.author_id().await)
            && (self
                .original_message()
                .content
                .to_lowercase()
                .starts_with("i'm ")
                || self
                    .original_message()
                    .content
                    .to_lowercase()
                    .starts_with("i am ")))
    }
    pub async fn keke_author(&self) -> Result<()> {
        let potential_keke = self
            .original_message()
            .content
            .strip_prefix("i'm ")
            .unwrap_or(&self.original_message().content)
            .strip_prefix("i am ")
            .unwrap_or(&self.original_message().content);
        if self.is_kekeable().await? {
            let name = self.author().name.clone();
            if self.original_message().content.chars().count() <= 32 {
                let member = self.member_request(self.author_id().await).await?;
                member
                    .edit(self.http_server(), |editmember| {
                        editmember.nickname(potential_keke)
                    })
                    .await?;
                self.send_message(format!(
                    "{name} is `{potential_keke}`!\n\nWanna optout? use {PREFIX}keke!"
                ))
                .await?;
            } else {
                self.send_message(format!(
                    "{name} is NOT `{potential_keke}`!\n\nWanna optout? use {PREFIX}keke!"
                ))
                .await?;
            }
            Ok(())
        } else {
            Err(SereneError::Other("Not a KEKE, ignorable").into())
        }
    }
    /// Gets the origin of a message. This is either [`MessageOrigin::PrivateChannel`]
    /// or [`MessageOrigin::PublicChannel`].
    pub fn message_origin(&self) -> MessageOrigin {
        if self.original_message().is_private() {
            MessageOrigin::PrivateChannel
        } else {
            MessageOrigin::PublicChannel
        }
    }
    pub fn guild_id(&self) -> SereneResult<u64> {
        self.original_message()
            .guild_id
            .ok_or(SereneError::Other("No guild id could be found"))
            .map(|x| x.0)
    }
}
