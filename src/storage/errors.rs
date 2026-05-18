use std::fmt;
use std::io;
use std::path::PathBuf;

#[derive(Debug)]
pub enum StorageError {
    MissingProjectDirectory,
    Io(io::Error),
    Json {
        path: PathBuf,
        source: serde_json::Error,
    },
}

impl fmt::Display for StorageError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingProjectDirectory => {
                write!(formatter, "could not find a suitable project directory")
            }
            Self::Io(error) => write!(formatter, "{error}"),
            Self::Json { path, source } => {
                write!(
                    formatter,
                    "failed to parse JSON file {}: {source}",
                    path.display()
                )
            }
        }
    }
}

impl std::error::Error for StorageError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::MissingProjectDirectory => None,
            Self::Io(error) => Some(error),
            Self::Json { source, .. } => Some(source),
        }
    }
}

impl From<io::Error> for StorageError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}
