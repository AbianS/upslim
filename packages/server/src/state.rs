use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};
use tracing::{debug, warn};

use crate::{error::Result, types::AlertState};

/// State key: "{monitor_name}:{provider_name}"
pub fn state_key(monitor_name: &str, provider_name: &str) -> String {
    format!("{monitor_name}:{provider_name}")
}

// ---------------------------------------------------------------------------
// StateStore — JSON file persisted to disk
// ---------------------------------------------------------------------------

/// AlertState store.
/// - In memory: `HashMap` protected by a `Mutex`
/// - On disk: JSON file at `state_dir/alert_state.json`
/// - Persisted after every `save()`
#[derive(Clone)]
pub struct StateStore {
    inner: Arc<Mutex<HashMap<String, AlertState>>>,
    file: PathBuf,
}

impl StateStore {
    /// Loads the existing state from disk (if present) or starts empty.
    pub fn load(state_dir: &Path) -> Result<Self> {
        std::fs::create_dir_all(state_dir)?;
        let file = state_dir.join("alert_state.json");

        let inner = if file.exists() {
            let content = std::fs::read_to_string(&file)?;
            match serde_json::from_str::<HashMap<String, AlertState>>(&content) {
                Ok(map) => {
                    debug!(path = %file.display(), entries = map.len(), "loaded alert state");
                    map
                }
                Err(e) => {
                    warn!(
                        path = %file.display(),
                        error = %e,
                        "alert state file is corrupt, starting fresh"
                    );
                    HashMap::new()
                }
            }
        } else {
            debug!(path = %file.display(), "no existing alert state, starting fresh");
            HashMap::new()
        };

        Ok(Self {
            inner: Arc::new(Mutex::new(inner)),
            file,
        })
    }

    /// Gets the state for a (monitor, provider) pair, or default if not present.
    pub fn get(&self, key: &str) -> AlertState {
        self.inner
            .lock()
            .expect("state lock poisoned")
            .get(key)
            .cloned()
            .unwrap_or_default()
    }

    /// Updates the state and persists it to disk.
    pub fn set(&self, key: &str, state: AlertState) {
        {
            let mut map = self.inner.lock().expect("state lock poisoned");
            map.insert(key.to_owned(), state);
        }
        self.persist();
    }

    /// Writes the full state to disk. Silences errors (a write failure should
    /// not bring down the server).
    fn persist(&self) {
        let map = self.inner.lock().expect("state lock poisoned").clone();
        match serde_json::to_string_pretty(&map) {
            Ok(json) => {
                if let Err(e) = std::fs::write(&self.file, json) {
                    warn!(path = %self.file.display(), error = %e, "failed to persist alert state");
                }
            }
            Err(e) => {
                warn!(error = %e, "failed to serialize alert state");
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn roundtrip_state() {
        let dir = tempdir().unwrap();
        let store = StateStore::load(dir.path()).unwrap();

        let key = state_key("api-health", "slack-ops");
        let mut state = AlertState::default();
        state.consecutive_failures = 3;
        state.is_firing = true;

        store.set(&key, state.clone());

        // Reload from disk
        let store2 = StateStore::load(dir.path()).unwrap();
        let loaded = store2.get(&key);

        assert_eq!(loaded.consecutive_failures, 3);
        assert!(loaded.is_firing);
    }

    #[test]
    fn missing_key_returns_default() {
        let dir = tempdir().unwrap();
        let store = StateStore::load(dir.path()).unwrap();
        let state = store.get("nonexistent:key");
        assert_eq!(state.consecutive_failures, 0);
        assert!(!state.is_firing);
    }

    #[test]
    fn corrupt_file_starts_fresh() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("alert_state.json");
        std::fs::write(&file, b"not json at all").unwrap();

        // Should load without error, empty state
        let store = StateStore::load(dir.path()).unwrap();
        let state = store.get("any:key");
        assert_eq!(state.consecutive_failures, 0);
    }

    #[test]
    fn state_key_format() {
        assert_eq!(state_key("my-monitor", "slack-ops"), "my-monitor:slack-ops");
    }
}
