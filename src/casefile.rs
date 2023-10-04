use crate::backend::{vec_str_to_string, BotShard, PREFIX};
use eyre::Result;
use serenity::Error as SereneError;
use std::{
    error::Error,
    fmt::Display,
    fs::{self as files, DirEntry},
    io::Error as IOError,
    num::ParseIntError,
    str::FromStr,
};
/// Represents an action pertaining to a Case File.
#[derive(Clone, PartialEq, Eq)]
pub enum CaseFileAction {
    /// Creates a new casefile
    Create {
        name: String,
    },
    /// Reads all of a casefile into chat as a summary.
    Read {
        id: u64,
    },
    /// Adds an item to a casefile.
    AddItem {
        id: u64,
        item: String,
    },
    ///
    RemoveItem {
        id: u64,
        index: Option<u64>,
    },
    Delete {
        id: u64,
    },
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
    /// Whether or not the relevant file exists or not.
    /// This should not be called for cases where this is
    /// [`CaseFileAction::Create`] or [`CaseFileAction::ViewAll`].
    pub fn file_exists(&self) -> Result<bool> {
        let id = match self.id() {
            Some(id) => id,
            None => return Ok(false),
        };
        for slot in files::read_dir("casefiles")? {
            let file = slot?;
            if file.file_name() == format!("{id}").as_str() {
                return Ok(true);
            }
        }
        Ok(false)
    }
    /// Finds the relevant directory based on the relevant ID.
    /// The "directory", in this case, is a [`DirEntry`].
    pub fn relevant_directory(&self) -> Result<DirEntry> {
        let id = self.id().ok_or(CaseFileError::IOError(IOError::new(
            std::io::ErrorKind::NotFound,
            "No relevant directory",
        )))?;
        for slot in files::read_dir("casefiles")? {
            let file = slot?;
            if file.file_name() == format!("{id}").as_str() {
                return Ok(file);
            }
        }
        Err(CaseFileError::IOError(IOError::new(
            std::io::ErrorKind::NotFound,
            "Could not find the corresponding entry",
        ))
        .into())
    }
    /// Whether or not any action is actually preformable on behalf of the caller.
    pub fn is_actionable(&self) -> Result<bool> {
        Ok(match self {
            CaseFileAction::Create { .. } => !self.file_exists()?,
            CaseFileAction::Read { .. }
            | CaseFileAction::AddItem { .. }
            | CaseFileAction::RemoveItem { .. }
            | CaseFileAction::Delete { .. } => self.file_exists()?,
            CaseFileAction::ViewAll => true,
        })
    }
    /// Gets the lowest ID availible for creating a case file.
    /// # Panics
    /// Panics if there are `u64::MAX` casefiles.
    pub fn lowest_id_availible() -> Result<u64, CaseFileError> {
        let ids = files::read_dir("casefiles")?
            .flat_map(|file| file.ok())
            .flat_map(|file| file.file_name().into_string().ok())
            .flat_map(|name| name.parse::<u64>())
            .collect::<Vec<_>>();
        for potential_id in 0.. {
            if !ids.contains(&potential_id) {
                return Ok(potential_id);
            }
        }
        unreachable!("If we have u64::MAX case files I think we need something better")
    }
    /// Executes the action using the given shard.
    /// TODO: implement Read, AddItem, RemoveItem, Delete, and ViewAll
    pub async fn execute(self, shard: BotShard<'_>) -> Result<()> {
        if self.is_actionable()? {
            match self {
                CaseFileAction::Create { name } => {
                    let id = Self::lowest_id_availible()?;
                    let path = format!("cafefiles\\{id}.txt");
                    files::write(path, format!("{name}|unresolved\n"))?;
                    shard
                        .send_message(format!(
                            "Successfully created file for '{name}'. Access it with id `{id}`."
                        ))
                        .await?;
                }
                CaseFileAction::Read { .. } => todo!(),
                CaseFileAction::AddItem { .. } => todo!(),
                CaseFileAction::RemoveItem { .. } => todo!(),
                CaseFileAction::Delete { .. } => todo!(),
                CaseFileAction::ViewAll => todo!(),
            }
        } else {
            shard
                .send_message("Oops! Looks like your casefile request was somehow malformed.")
                .await?;
        }
        Ok(())
    }
}

impl FromStr for CaseFileAction {
    type Err = CaseFileError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let args = s.split(|chr| chr == ' ' || chr == '\n').collect::<Vec<_>>();
        if args.is_empty() || args[0] != "casefuile" {
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
/// an example file might look like this:
/// ```txt
/// [casefiles/1.txt]
/// Foo v. Bar|resolved
/// - Foo hates bar we thing
/// - Nvm they're just gay
/// - [link to kissing video]
/// ```
/// This format should be followed for the [FromStr] implementation to succeed.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct CaseFile {
    name: String,
    resolved: bool,
    items: Vec<String>,
}

impl CaseFile {
    /// Clones the name associated with the CaseFile
    pub fn name(&self) -> String {
        self.name.clone()
    }
    /// Gets whether the case is considered resolved
    pub fn is_resolved(&self) -> bool {
        self.resolved
    }
    /// Clones the evidence items associated with the CaseFile
    pub fn items(&self) -> Vec<String> {
        self.items.clone()
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
    /// There was some error when parsing a [`CaseFile`] or [`CaseFileAction`]
    ParsingError(String),
    /// An [IOError] was raised during file interaction.
    IOError(IOError),
    /// [`serenity`] raised an error when using the [`BotShard`].
    SerenityError(SereneError),
}

impl Display for CaseFileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
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
