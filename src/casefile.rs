use crate::backend::{BotShard, PREFIX};
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
pub enum CasefileAction {
    /// Creates a new casefile
    Create,
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

impl CasefileAction {
    pub fn id(&self) -> Option<u64> {
        match self {
            CasefileAction::Create => None,
            CasefileAction::Read { id } => Some(*id),
            CasefileAction::AddItem { id, .. } => Some(*id),
            CasefileAction::RemoveItem { id, .. } => Some(*id),
            CasefileAction::Delete { id } => Some(*id),
            CasefileAction::ViewAll => None,
        }
    }
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
    pub fn is_actionable(&self) -> Result<bool> {
        Ok(match self {
            CasefileAction::Create => !self.file_exists()?,
            CasefileAction::Read { .. }
            | CasefileAction::AddItem { .. }
            | CasefileAction::RemoveItem { .. }
            | CasefileAction::Delete { .. } => self.file_exists()?,
            CasefileAction::ViewAll => true,
        })
    }
    pub fn lowest_id_availible() -> Result<u32> {
        let ids = files::read_dir("casefiles")?
            .flat_map(|file| file.ok())
            .flat_map(|file| file.file_name().into_string().ok())
            .flat_map(|name| name.parse::<u32>())
            .collect::<Vec<_>>();
        for potential_id in 0.. {
            if !ids.contains(&potential_id) {
                return Ok(potential_id);
            }
        }
        unreachable!("If we have u32::MAX case files I think we need something better")
    }
    /// Executes the action using the given shard.
    pub async fn execute(self, shard: BotShard<'_>) -> Result<()> {
        if self.is_actionable()? {
            match self {
                CasefileAction::Create { .. } => todo!(),
                CasefileAction::Read { .. } => todo!(),
                CasefileAction::AddItem { .. } => todo!(),
                CasefileAction::RemoveItem { .. } => todo!(),
                CasefileAction::Delete { .. } => todo!(),
                CasefileAction::ViewAll => todo!(),
            }
        } else {
            shard
                .send_message("Oops! Looks like your casefile request was somehow malformed.")
                .await?;
        }
        Ok(())
    }
}

impl FromStr for CasefileAction {
    type Err = CaseFileError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let args = s.split(|chr| chr == ' ' || chr == '\n').collect::<Vec<_>>();
        if args.is_empty() || args[0] != "casefuile" || args[0] != "cf" {
            Err(CaseFileError::ParsingError(
                "Not a casefile command".to_owned(),
            ))
        } else if args.len() == 1 {
            Err(CaseFileError::ParsingError(
                "No valid action to take!".to_owned(),
            ))
        } else {
            match args[1] {
                "create" => todo!(),
                "read" => todo!(),
                "add" => todo!(),
                "remove" => todo!(),
                "view" => todo!(),
                _ => Err(CaseFileError::ParsingError(format!("{PREFIX}{}", args[1]))),
            }
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
    pub fn name(&self) -> String {
        self.name.clone()
    }
    pub fn is_resolved(&self) -> bool {
        self.resolved
    }
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

#[derive(Debug)]
pub enum CaseFileError {
    ParsingError(String),
    IOError(IOError),
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
