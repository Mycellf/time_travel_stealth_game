use core::fmt;
use std::{
    fmt::{Display, Formatter},
    fs, io,
    path::{Path, PathBuf},
};

use include_dir::{Dir, include_dir};

#[derive(Clone)]
pub enum FileSystem {
    Direct { root: PathBuf },
    Stored { files: Dir<'static> },
}

pub const LEVELS_DIRECTORY: &str = "resources/levels";
pub const STORED_LEVELS: Dir = include_dir!("resources/levels");

impl Default for FileSystem {
    fn default() -> Self {
        if Path::new(LEVELS_DIRECTORY).exists() {
            FileSystem::Direct {
                root: PathBuf::from(LEVELS_DIRECTORY),
            }
        } else {
            FileSystem::Stored {
                files: STORED_LEVELS,
            }
        }
    }
}

impl FileSystem {
    pub fn load(&self, level: &str) -> Result<Vec<u8>, LoadLevelError> {
        match self {
            FileSystem::Direct { root } => {
                let mut path = root.clone();
                path.push(level);

                fs::read(path).map_err(|error| LoadLevelError::IoError(error))
            }
            FileSystem::Stored { files } => {
                if let Some(file) = files.get_file(level) {
                    Ok(file.contents().to_owned())
                } else {
                    Err(LoadLevelError::NoSuchLevel)
                }
            }
        }
    }

    pub fn save(&self, level: &str, contents: &[u8]) -> Result<(), SaveLevelError> {
        match self {
            FileSystem::Direct { root } => {
                let mut path = root.clone();
                path.push(level);

                fs::write(path, contents).map_err(|error| SaveLevelError::IoError(error))
            }
            FileSystem::Stored { .. } => Err(SaveLevelError::Unsupported),
        }
    }
}

#[derive(Debug)]
pub enum LoadLevelError {
    NoSuchLevel,
    IoError(io::Error),
}

impl Display for LoadLevelError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            LoadLevelError::NoSuchLevel => write!(f, "No such level to load",),
            LoadLevelError::IoError(error) => write!(f, "{error}"),
        }
    }
}

#[derive(Debug)]
pub enum SaveLevelError {
    Unsupported,
    IoError(io::Error),
}

impl Display for SaveLevelError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            SaveLevelError::Unsupported => write!(
                f,
                "Saving is unsupported without access to a \"resources/levels\" directory",
            ),
            SaveLevelError::IoError(error) => write!(f, "{error}"),
        }
    }
}
