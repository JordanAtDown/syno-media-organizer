use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use tracing::warn;

#[derive(Debug, Serialize, Deserialize, Clone)]
struct CacheEntry {
    mtime: u64,
    cached_at: u64,
}

/// Persistent cache of files that had no capture date.
///
/// Stored as JSON at `path`. Loaded once at startup, written only when the
/// set of entries changes. Entries are invalidated when the file's mtime
/// changes (file was modified) or when the configured TTL expires.
pub struct NoDateCache {
    path: PathBuf,
    enabled: bool,
    ttl_secs: u64,
    entries: HashMap<PathBuf, CacheEntry>,
    dirty: bool,
}

fn now_unix_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

impl NoDateCache {
    /// Load cache from disk. If the file does not exist, start with an empty cache.
    /// Entries for files that no longer exist on disk, or whose TTL has expired,
    /// are pruned on load.
    pub fn load(path: PathBuf, enabled: bool, ttl_days: u32) -> Self {
        let ttl_secs = (ttl_days as u64) * 86400;

        if !enabled {
            return Self {
                path,
                enabled: false,
                ttl_secs,
                entries: HashMap::new(),
                dirty: false,
            };
        }

        let raw: HashMap<PathBuf, CacheEntry> = path
            .exists()
            .then(|| std::fs::read_to_string(&path).ok())
            .flatten()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default();

        let now = now_unix_secs();
        let entries: HashMap<PathBuf, CacheEntry> = raw
            .into_iter()
            .filter(|(p, entry)| {
                if !p.exists() {
                    return false;
                }
                if ttl_secs > 0 && entry.cached_at + ttl_secs < now {
                    return false;
                }
                true
            })
            .collect();

        Self {
            path,
            enabled,
            ttl_secs,
            entries,
            dirty: false,
        }
    }

    /// Returns `true` if the file is in the cache and its mtime hasn't changed
    /// and its TTL hasn't expired — i.e., it should be silently skipped.
    pub fn should_skip(&self, file: &Path, current_mtime_secs: u64) -> bool {
        if !self.enabled {
            return false;
        }
        match self.entries.get(file) {
            Some(entry) => {
                if entry.mtime != current_mtime_secs {
                    return false;
                }
                if self.ttl_secs > 0 && entry.cached_at + self.ttl_secs < now_unix_secs() {
                    return false;
                }
                true
            }
            None => false,
        }
    }

    /// Record a file as having no capture date. Marks the cache dirty.
    pub fn insert(&mut self, file: PathBuf, mtime_secs: u64) {
        if !self.enabled {
            return;
        }
        self.entries.insert(
            file,
            CacheEntry {
                mtime: mtime_secs,
                cached_at: now_unix_secs(),
            },
        );
        self.dirty = true;
    }

    /// Remove a file from the cache (e.g. it was successfully processed after
    /// being modified). No-op if the file was not cached.
    pub fn remove(&mut self, file: &Path) {
        if self.entries.remove(file).is_some() {
            self.dirty = true;
        }
    }

    /// Flush the cache to disk if it has changed since the last save.
    pub fn save_if_dirty(&mut self) {
        if !self.dirty || !self.enabled {
            return;
        }
        match serde_json::to_string_pretty(&self.entries) {
            Ok(json) => {
                if let Err(e) = std::fs::write(&self.path, json) {
                    warn!(path = %self.path.display(), error = %e, "failed to write no-date cache");
                } else {
                    self.dirty = false;
                }
            }
            Err(e) => {
                warn!(error = %e, "failed to serialize no-date cache");
            }
        }
    }
}
