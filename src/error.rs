use thiserror::Error;

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("Failed to read config file: {0}")]
    Io(#[from] std::io::Error),
    #[error("Failed to parse config file: {0}")]
    Parse(#[from] toml::de::Error),
    #[error("Invalid configuration: {0}")]
    Invalid(String),
}

#[derive(Debug, Error)]
pub enum ExifError {
    #[error("Failed to read file: {0}")]
    Io(#[from] std::io::Error),
    #[error("Failed to parse EXIF data: {0}")]
    Parse(String),
    #[error("Failed to write EXIF data: {0}")]
    Write(String),
}

#[derive(Debug, Error)]
pub enum NamingError {
    #[error("Invalid pattern token: {0}")]
    InvalidToken(String),
    #[error("File conflict cannot be resolved: {0}")]
    ConflictUnresolvable(String),
}

#[derive(Debug, Error)]
pub enum ProcessorError {
    #[error("File not found: {0}")]
    FileNotFound(String),
    #[error("Extension not allowed: {0}")]
    ExtensionNotAllowed(String),
    #[error("IO error during processing: {0}")]
    Io(#[from] std::io::Error),
    #[error("Naming error: {0}")]
    Naming(#[from] NamingError),
}

#[derive(Debug, Error)]
pub enum WatcherError {
    #[error("Failed to initialize watcher: {0}")]
    Init(String),
    #[error("Watch error: {0}")]
    Watch(String),
}
