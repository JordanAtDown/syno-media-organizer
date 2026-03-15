use crate::error::ConfigError;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Default)]
#[serde(rename_all = "snake_case")]
pub enum OnConflict {
    #[default]
    Rename,
    Skip,
    Overwrite,
}

fn default_pattern() -> String {
    "{year}/{month}/{year}-{month}-{day}_{hour}{min}{sec}_{stem}{ext}".to_string()
}

fn default_extensions() -> Vec<String> {
    vec![
        "jpg".to_string(),
        "jpeg".to_string(),
        "png".to_string(),
        "heic".to_string(),
        "mp4".to_string(),
        "mov".to_string(),
        "avi".to_string(),
        "mkv".to_string(),
    ]
}

fn default_true() -> bool {
    true
}

fn default_poll_interval() -> u64 {
    30
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct FolderConfig {
    /// Source folder to watch
    pub input: PathBuf,
    /// Destination root folder
    pub output: PathBuf,
    /// Naming pattern using tokens like {year}, {month}, {day}, {stem}, {ext}
    #[serde(default = "default_pattern")]
    pub pattern: String,
    /// Watch subdirectories recursively
    #[serde(default = "default_true")]
    pub recursive: bool,
    /// Move files (true) or copy them (false)
    #[serde(default = "default_true")]
    pub move_files: bool,
    /// What to do when destination file already exists
    #[serde(default)]
    pub on_conflict: OnConflict,
    /// Allowed file extensions (lowercase, without dot)
    #[serde(default = "default_extensions")]
    pub extensions: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Config {
    #[serde(rename = "folders")]
    pub folders: Vec<FolderConfig>,
    /// How often to scan input folders, in seconds (default: 30)
    #[serde(default = "default_poll_interval")]
    pub poll_interval_secs: u64,
}

pub fn load(path: &Path) -> Result<Config, ConfigError> {
    let content = std::fs::read_to_string(path)?;
    let config: Config = toml::from_str(&content)?;

    validate(&config)?;

    Ok(config)
}

fn validate(config: &Config) -> Result<(), ConfigError> {
    if config.folders.is_empty() {
        return Err(ConfigError::Invalid(
            "At least one [[folders]] entry is required".to_string(),
        ));
    }

    for folder in &config.folders {
        if folder.input == folder.output {
            return Err(ConfigError::Invalid(format!(
                "Input and output folders must differ: {}",
                folder.input.display()
            )));
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    fn write_config(content: &str) -> NamedTempFile {
        let mut f = NamedTempFile::new().unwrap();
        write!(f, "{}", content).unwrap();
        f
    }

    #[test]
    fn test_parse_valid_config() {
        let f = write_config(
            r#"
[[folders]]
input = "/volume1/inbox"
output = "/volume1/photos"
"#,
        );
        let cfg = load(f.path()).unwrap();
        assert_eq!(cfg.folders.len(), 1);
        assert_eq!(cfg.folders[0].input, PathBuf::from("/volume1/inbox"));
        assert_eq!(cfg.folders[0].output, PathBuf::from("/volume1/photos"));
        assert!(cfg.folders[0].move_files);
        assert!(cfg.folders[0].recursive);
        assert_eq!(cfg.folders[0].on_conflict, OnConflict::Rename);
    }

    #[test]
    fn test_parse_multiple_folders() {
        let f = write_config(
            r#"
[[folders]]
input = "/volume1/inbox/camera"
output = "/volume1/photos"
pattern = "{year}/{month}/{stem}{ext}"
move_files = false
on_conflict = "skip"

[[folders]]
input = "/volume1/inbox/phone"
output = "/volume1/photos"
recursive = false
"#,
        );
        let cfg = load(f.path()).unwrap();
        assert_eq!(cfg.folders.len(), 2);
        assert!(!cfg.folders[0].move_files);
        assert_eq!(cfg.folders[0].on_conflict, OnConflict::Skip);
        assert!(!cfg.folders[1].recursive);
    }

    #[test]
    fn test_reject_empty_folders() {
        let f = write_config("[folders]\n");
        assert!(load(f.path()).is_err());
    }

    #[test]
    fn test_reject_same_input_output() {
        let f = write_config(
            r#"
[[folders]]
input = "/volume1/photos"
output = "/volume1/photos"
"#,
        );
        assert!(load(f.path()).is_err());
    }

    #[test]
    fn test_parse_invalid_toml() {
        let f = write_config("this is not toml ::::");
        assert!(load(f.path()).is_err());
    }

    #[test]
    fn test_custom_extensions() {
        let f = write_config(
            r#"
[[folders]]
input = "/volume1/inbox"
output = "/volume1/photos"
extensions = ["jpg", "png"]
"#,
        );
        let cfg = load(f.path()).unwrap();
        assert_eq!(cfg.folders[0].extensions, vec!["jpg", "png"]);
    }
}
