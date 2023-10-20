//! Deals with casefiles, abstracted with [`Casefile`] structs.

use crate::backend::{vec_str_to_string, vec_string_to_string, PREFIX};
use crate::shard::BotShard;
use eyre::Result;
use rusqlite as sql;
use serenity::Error as SereneError;
use std::ops::{Deref, DerefMut};
use std::{error::Error, fmt::Display, io::Error as IOError, num::ParseIntError, str::FromStr};

/// Points to the file that should be used for the internal SQL database
pub const DATABASE_FILE: &str = "./db.db3";
/// Represents an action pertaining to a Case File.
#[derive(Clone, PartialEq, Eq)]
pub enum CaseFileAction {
    /// Creates a new casefile
    Create {
        #[doc = "the name of the case"]
        name: String,
    },
    /// Reads all of a casefile into chat as a summary.
    Read {
        #[doc = "the relevant id"]
        id: u64,
    },
    /// Adds an item to a casefile.
    AddItem {
        #[doc = "the relevant id"]
        id: u64,
        #[doc = "the item to add to the file"]
        item: String,
    },
    /// Removes an item from a casefile
    RemoveItem {
        #[doc = "the relevant id"]
        id: u64,
        #[doc = "docs"]
        index: Option<u64>,
    },
    /// Deletes a casefile
    Delete {
        #[doc = "the relevant id"]
        id: u64,
    },
    /// Views a summary of all casefiles
    ViewAll,
}

impl CaseFileAction {
    /// Gets the relevant casefile ID, if one is present.
    pub fn id(&self) -> Option<u64> {
        match self {
            CaseFileAction::Create { .. } => None,
            CaseFileAction::Read { id } => Some(*id),
            CaseFileAction::AddItem { id, .. } => Some(*id),
            CaseFileAction::RemoveItem { id, .. } => Some(*id),
            CaseFileAction::Delete { id } => Some(*id),
            CaseFileAction::ViewAll => None,
        }
    }
    /// Gets the lowest ID availible for creating a case file.
    /// # Panics
    /// Panics if there are `u64::MAX` casefiles.
    pub fn lowest_id_availible() -> Result<u64> {
        let db = query_database()?;
        let mut id = 0;
        // intentional ignore of () iterator
        let _ = db
            .prepare("SELECT TOP 1 FROM cases")?
            .query_map((), |row| {
                let x = row.get::<_, u64>(0)?;
                id = id.max(x);
                Ok(())
            })?;
        Ok(id)
    }
    /// Executes the action using the given shard.
    pub async fn execute(self, shard: BotShard<'_>) -> Result<()> {
        match self {
            CaseFileAction::Create { name } => {
                let id = Self::lowest_id_availible()?;
                let db = query_database()?;
                db.prepare(
                    "
                        INSERT INTO cases (id, name, reso, data)
                        VALUES ((?1), (?2), (?3), (?4))
                    ",
                )?
                .execute((&id, &name, false, ""))?;
                shard
                    .send_message(format!(
                        "Successfully created file for '{name}'. Access it with id `{id}`."
                    ))
                    .await?;
            }
            CaseFileAction::Read { id } => {
                let file = CaseFile::from_id(id)?;
                let items = file
                    .items
                    .clone()
                    .iter_mut()
                    .flat_map(|string| {
                        string.push_str("\n> ");
                        string.chars()
                    })
                    .collect::<String>();
                let readable = format!("Case #{id} => {}\n{items}", file.name);
                shard.send_message(readable).await?;
            }
            CaseFileAction::AddItem { id, item } => {
                let mut file = CaseFile::from_id(id)?;
                file.push_item(item);
                file.write_to_id(id)?;
                shard
                    .send_message(format!("Successfully wrote new item to Casefile #{id}!"))
                    .await?;
            }
            CaseFileAction::RemoveItem { id, index } => {
                let mut file = CaseFile::from_id(id)?;
                let item = match index {
                    Some(idx) => Some(file.items.remove(idx as usize)),
                    None => file.items.pop(),
                }
                .unwrap_or("[unable to find item]".to_owned());
                file.write_to_id(id)?;
                shard
                    .send_message(format!("Removed item `{item}` from Casefile #{id}."))
                    .await?;
            }
            CaseFileAction::Delete { id } => {
                let db = query_database()?;
                db.prepare(
                    "
                        DELETE FROM cases WHERE id = (?1)
                    ",
                )?
                .execute((&id,))?;
                shard
                    .send_message(format!("Successfully removed Casefile #{id}."))
                    .await?;
            }
            CaseFileAction::ViewAll => {
                let mut buffer = String::from("Here's all the casefiles: \n");
                for file in CaseFile::all_files() {
                    buffer.push_str(format!("[{}] | {}\n", file.resolution(), file.name).as_str());
                }
                shard.send_message(buffer).await?;
            }
        }

        Ok(())
    }
}

impl FromStr for CaseFileAction {
    type Err = CaseFileError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let args = s.split(|chr| chr == ' ' || chr == '\n').collect::<Vec<_>>();
        if args.is_empty() || args[0] != "casefile" {
            Err(CaseFileError::ParsingError(
                "Not a casefile command".to_owned(),
            ))
        } else if args.len() == 1 {
            Err(CaseFileError::ParsingError(
                "No valid action to take!".to_owned(),
            ))
        } else {
            Ok(match args[1] {
                "create" => CaseFileAction::Create {
                    name: vec_str_to_string(&args, Some(2)),
                },
                "read" => CaseFileAction::Read {
                    id: {
                        if args.len() < 3 {
                            return Err(CaseFileError::ParsingError(
                                "no given index to read from".to_owned(),
                            ));
                        } else {
                            args[2].parse()?
                        }
                    },
                },
                "add" => CaseFileAction::AddItem {
                    id: {
                        if args.len() < 3 {
                            return Err(CaseFileError::ParsingError(
                                "no given index to add to".to_owned(),
                            ));
                        } else {
                            args[2].parse()?
                        }
                    },
                    item: if args.len() < 4 {
                        return Err(CaseFileError::ParsingError("no item to add".to_owned()));
                    } else {
                        vec_str_to_string(&args, Some(3))
                    },
                },
                "remove" => CaseFileAction::RemoveItem {
                    id: if args.len() < 3 {
                        return Err(CaseFileError::ParsingError(
                            "no given index to read from".to_owned(),
                        ));
                    } else {
                        args[2].parse()?
                    },
                    index: if args.len() < 3 {
                        None
                    } else {
                        Some(vec_str_to_string(&args, Some(2)).parse()?)
                    },
                },
                "view" => CaseFileAction::ViewAll,
                _ => return Err(CaseFileError::ParsingError(format!("{PREFIX}{}", args[1]))),
            })
        }
    }
}
/// A representation of a case file.
/// This format should be followed for the [FromStr] implementation to succeed.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct CaseFile {
    /// The name of the casefile
    pub name: String,
    /// Whether or not the casefile is resolved (true = resolved)
    pub resolved: bool,
    /// The related evidence or other noteworthy items
    pub items: Vec<String>,
}

impl CaseFile {
    /// Gets whether the case is considered resolved
    pub fn is_resolved(&self) -> bool {
        self.resolved
    }
    /// Gets the resolution as a string (either `"resolved"` or `"unresolved"`).
    pub fn resolution(&self) -> String {
        match self.is_resolved() {
            true => "resolved",
            false => "unresolved",
        }
        .to_owned()
    }
    /// Attempts to write a new item to this casefile
    pub fn push_item(&mut self, item: impl AsRef<str>) {
        self.items.push(item.as_ref().to_owned());
    }
    /// Attempts to get a casefile given an ID.
    pub fn from_id(id: u64) -> Result<CaseFile> {
        let db = query_database()?;
        let mut statement =
            db.prepare(format!("SELECT name, reso, data FROM cases WHERE id = {id}").as_str())?;
        let mut case = statement.query_map([], |row| {
            let name = row.get::<_, String>(0)?;
            let resolved = row.get::<_, bool>(1)?;
            let items = row
                .get::<_, String>(2)?
                .lines()
                .map(ToOwned::to_owned)
                .collect::<Vec<_>>();
            Ok(CaseFile {
                name,
                resolved,
                items,
            })
        })?;
        let case = case.next().ok_or_else(|| {
            CaseFileError::ParsingError("Couldn't get the case from the SQL database".to_owned())
        })??;
        Ok(case)
    }
    /// Gets an iterator of all the stored casefiles.
    /// Any errors returned are thrown out.
    pub fn all_files() -> impl Iterator<Item = Self> {
        (0..CaseFileAction::lowest_id_availible().unwrap_or_default()).flat_map(Self::from_id)
    }
    /// Writes the contents of this casefile to the relevant id.
    pub fn write_to_id(&self, id: u64) -> Result<()> {
        let db = query_database()?;
        let data = vec_string_to_string(&self.items, None);
        db.prepare(
            "
            UPDATE cases
            SET data = (?1)
            WHERE id = (?2)
        ",
        )?
        .execute((&id, &data))?;
        Ok(())
    }
}

impl Display for CaseFile {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let items = self
            .items
            .iter()
            .flat_map(|string| string.chars().chain(std::iter::once('\n')))
            .collect::<String>();
        let resolution = match self.is_resolved() {
            true => "resolved",
            false => "unresolved",
        };
        write!(f, "{}|{resolution}\n{items}", self.name)
    }
}

impl FromStr for CaseFile {
    type Err = CaseFileError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (name, rest) = s.split_once('|').ok_or(CaseFileError::ParsingError(
            "No specification for resolution status".to_owned(),
        ))?;
        let (resolution, items) = rest.split_once('\n').ok_or(CaseFileError::ParsingError(
            "Must be a newline after the resolution status".to_owned(),
        ))?;
        let resolved = match resolution {
            "resolved" => true,
            "unresolved" => false,
            _ => {
                return Err(CaseFileError::ParsingError(
                    "resolution does not match 'resolved' or 'unresolved'".to_owned(),
                ));
            }
        };
        let items = items.split("\n- ").map(str::to_owned).collect();
        Ok(CaseFile {
            name: name.to_owned(),
            resolved,
            items,
        })
    }
}

/// Represents a number of errors that can occur from interacting with [`CaseFile`]s.
#[derive(Debug)]
pub enum CaseFileError {
    /// There was some error when parsing a [`CaseFile`] or [`CaseFileAction`].
    /// [`ParseIntError`]s get turned into this variant.
    ParsingError(String),
    /// An [IOError] was raised during file interaction.
    IOError(IOError),
    /// [`serenity`] raised an error when using the [`BotShard`].
    SerenityError(SereneError),
}

impl Display for CaseFileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CaseFileError::ParsingError(e) => write!(f, "parsing error: {e}"),
            CaseFileError::IOError(e) => write!(f, "io error: {e}"),
            CaseFileError::SerenityError(e) => write!(f, "discord-originating error: {e}"),
        }
    }
}

impl Error for CaseFileError {}

impl From<IOError> for CaseFileError {
    fn from(value: IOError) -> Self {
        Self::IOError(value)
    }
}

impl From<SereneError> for CaseFileError {
    fn from(value: SereneError) -> Self {
        Self::SerenityError(value)
    }
}

impl From<ParseIntError> for CaseFileError {
    fn from(value: ParseIntError) -> Self {
        Self::ParsingError(format!("{value}"))
    }
}

/// Represents a connection to the internal database.
pub struct Database(sql::Connection);

impl Deref for Database {
    type Target = sql::Connection;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Database {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

/// Attempts to connect to the database file.
pub fn query_database() -> Result<Database, sql::Error> {
    Ok(Database(sql::Connection::open(DATABASE_FILE)?))
}

/// Attempts to create and inditalize the database.
/// Only does so if the database exists
pub fn create_database() -> Result<(), sql::Error> {
    // if file doesn't exist
    if std::fs::File::open(DATABASE_FILE).is_err() {
        let db = query_database()?;
        db.execute(
            "
            CREATE TABLE users (
                id   INTEGER PRIMARY KEY
                keke BOOLEAN
                blck BOOLEAN
            )
            ",
            (),
        )?;
        db.execute(
            "
            CREATE TABLE cases (
                id   INTEGER PRIMARY KEY
                name TINYTEXT
                reso BOOLEAN
                data LONGTEXT
            )
            ",
            (),
        )?;
    }
    Ok(())
}
