use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use directories::ProjectDirs;
use outpost_core as core;
use serde::{Deserialize, Serialize};

#[derive(thiserror::Error, Debug)]
pub enum StorageError {
    #[error("io error: {0}")]
    Io(String),
    #[error("serialize error: {0}")]
    Serialize(String),
    #[error("not found: {0}")]
    NotFound(String),
    #[error("unsupported on this platform: {0}")]
    Unsupported(&'static str),
}

impl From<io::Error> for StorageError {
    fn from(e: io::Error) -> Self { StorageError::Io(e.to_string()) }
}

pub type Result<T, E = StorageError> = std::result::Result<T, E>;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SaveProfile {
    pub id: String,
    pub label: String,
}

pub trait Storage: Send + Sync + 'static {
    fn list_profiles(&self) -> Result<Vec<SaveProfile>>;
    fn load(&self, profile_id: &str) -> Result<core::GameState>;
    fn save(&self, profile_id: &str, state: &core::GameState) -> Result<()>;
}

// ---- Save format versioning ----
const SAVE_FORMAT_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SaveFile {
    version: u32,
    #[serde(flatten)]
    payload: SavePayload,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind")]
enum SavePayload {
    #[serde(rename = "gamestate_v1")]
    GameStateV1 { state: core::GameState },
}

fn migrate_save(version: u32, payload: SavePayload) -> Result<core::GameState> {
    match (version, payload) {
        (1, SavePayload::GameStateV1 { state }) => Ok(state),
        (v, _) => Err(StorageError::Serialize(format!(
            "unsupported save version {}", v
        ))),
    }
}

fn encode_state(state: &core::GameState) -> Result<String> {
    let save = SaveFile {
        version: SAVE_FORMAT_VERSION,
        payload: SavePayload::GameStateV1 { state: state.clone() },
    };
    serde_json::to_string_pretty(&save).map_err(|e| StorageError::Serialize(e.to_string()))
}

fn decode_state(s: &str) -> Result<core::GameState> {
    // First try the new wrapper format
    match serde_json::from_str::<SaveFile>(s) {
        Ok(save) => migrate_save(save.version, save.payload),
        Err(_) => {
            // Backward compatibility: try raw GameState
            core::game_state_from_json(s).map_err(|e| StorageError::Serialize(e.to_string()))
        }
    }
}

// Desktop filesystem storage (JSON) — default under `desktop` feature
#[cfg(feature = "desktop")]
pub struct FsStorage {
    root: PathBuf,
}

#[cfg(feature = "desktop")]
impl FsStorage {
    pub fn new_default() -> Result<Self> {
        let proj = ProjectDirs::from("com", "Outpost", "Outpost3")
            .ok_or_else(|| StorageError::Io("no project dirs".into()))?;
        let dir = proj.data_local_dir();
        fs::create_dir_all(dir)?;
        Ok(Self { root: dir.to_path_buf() })
    }

    pub fn with_root_dir<P: AsRef<Path>>(p: P) -> Result<Self> {
        let p = p.as_ref().to_path_buf();
        fs::create_dir_all(&p)?;
        Ok(Self { root: p })
    }

    fn profile_path(&self, profile_id: &str) -> PathBuf {
        self.root.join(format!("{}.json", profile_id))
    }
}

#[cfg(feature = "desktop")]
impl Storage for FsStorage {
    fn list_profiles(&self) -> Result<Vec<SaveProfile>> {
        let mut out = Vec::new();
        for entry in fs::read_dir(&self.root)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_file() {
                if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                    out.push(SaveProfile { id: stem.to_string(), label: stem.to_string() });
                }
            }
        }
        out.sort_by(|a, b| a.id.cmp(&b.id));
        Ok(out)
    }

    fn load(&self, profile_id: &str) -> Result<core::GameState> {
        let p = self.profile_path(profile_id);
        if !p.exists() { return Err(StorageError::NotFound(profile_id.to_string())); }
        let s = fs::read_to_string(p)?;
        decode_state(&s)
    }

    fn save(&self, profile_id: &str, state: &core::GameState) -> Result<()> {
        let p = self.profile_path(profile_id);
        let s = encode_state(state)?;
        fs::write(p, s)?;
        Ok(())
    }
}

// WASM storage using IndexedDB with a synchronous LocalStorage cache.
// Notes:
// - Storage trait is sync, so writes are mirrored to LocalStorage immediately
//   and also dispatched asynchronously to IndexedDB.
// - Reads prefer the LocalStorage cache; an async refresh from IndexedDB is
//   kicked off in the background to keep the cache warm.
// - This keeps UI responsive while providing durable storage in IndexedDB.
#[cfg(feature = "wasm")]
pub struct WasmIdbStorage;

#[cfg(feature = "wasm")]
impl WasmIdbStorage {
    const PREFIX: &'static str = "outpost_save_";
    const DB_NAME: &'static str = "outpost3_db";
    const STORE_NAME: &'static str = "saves";

    fn local_storage() -> Result<web_sys::Storage> {
        let win = web_sys::window().ok_or_else(|| StorageError::Io("no window".into()))?;
        win.local_storage()
            .map_err(|e| StorageError::Io(format!("localStorage error: {:?}", e)))?
            .ok_or_else(|| StorageError::Io("no localStorage".into()))
    }

    fn cache_key(profile_id: &str) -> String { format!("{}{}", Self::PREFIX, profile_id) }

    // Async: ensure DB open and object store exists
    async fn idb_open() -> std::result::Result<indexed_db_futures::IdbDatabase, String> {
        use indexed_db_futures::prelude::*;
        let mut open_req = IdbDatabase::open_u32(Self::DB_NAME, 1)
            .map_err(|e| format!("open db error: {:?}", e))?;
        open_req.set_on_upgrade_needed(|evt| {
            let db = evt.db();
            if db.object_store_names().find(|n| n == Self::STORE_NAME).is_none() {
                let _ = db.create_object_store(Self::STORE_NAME);
            }
        });
        open_req.await.map_err(|e| format!("await open db: {:?}", e))
    }

    async fn idb_put(profile_id: &str, value: String) -> std::result::Result<(), String> {
        use indexed_db_futures::prelude::*;
        let db = Self::idb_open().await?;
        let tx = db
            .transaction_on_one_with_mode(Self::STORE_NAME, IdbTransactionMode::Readwrite)
            .map_err(|e| format!("tx error: {:?}", e))?;
        let store = tx.store(Self::STORE_NAME).map_err(|e| format!("store err: {:?}", e))?;
        store.put_key_val_owned(&profile_id.into(), &value.into())
            .map_err(|e| format!("put err: {:?}", e))?;
        tx.done().await.map_err(|e| format!("tx done err: {:?}", e))
    }

    async fn idb_get(profile_id: &str) -> std::result::Result<Option<String>, String> {
        use indexed_db_futures::prelude::*;
        let db = Self::idb_open().await?;
        let tx = db
            .transaction_on_one_with_mode(Self::STORE_NAME, IdbTransactionMode::Readonly)
            .map_err(|e| format!("tx error: {:?}", e))?;
        let store = tx.store(Self::STORE_NAME).map_err(|e| format!("store err: {:?}", e))?;
        let res = store.get_owned(&profile_id.into())
            .map_err(|e| format!("get err: {:?}", e))?
            .await
            .map_err(|e| format!("await get err: {:?}", e))?;
        let s = res.and_then(|v| v.as_string());
        tx.done().await.ok();
        Ok(s)
    }

    // Fire-and-forget helpers
    fn spawn_write_to_idb(profile_id: String, value: String) {
        wasm_bindgen_futures::spawn_local(async move {
            let _ = Self::idb_put(&profile_id, value).await;
        });
    }

    fn spawn_refresh_cache_from_idb(profile_id: String) {
        wasm_bindgen_futures::spawn_local(async move {
            if let Ok(Some(val)) = Self::idb_get(&profile_id).await {
                if let Ok(storage) = Self::local_storage() {
                    let _ = storage.set_item(&Self::cache_key(&profile_id), &val);
                }
            }
        });
    }
}

#[cfg(feature = "wasm")]
impl Storage for WasmIdbStorage {
    fn list_profiles(&self) -> Result<Vec<SaveProfile>> {
        // Use LocalStorage cache for synchronous listing
        let storage = Self::local_storage()?;
        let len = storage.length().map_err(|e| StorageError::Io(format!("len err: {:?}", e)))?;
        let mut out = Vec::new();
        for i in 0..len {
            if let Ok(Some(key)) = storage.key(i) {
                if let Some(stripped) = key.strip_prefix(Self::PREFIX) {
                    out.push(SaveProfile { id: stripped.to_string(), label: stripped.to_string() });
                }
            }
        }
        out.sort_by(|a, b| a.id.cmp(&b.id));
        Ok(out)
    }

    fn load(&self, profile_id: &str) -> Result<core::GameState> {
        // Try LocalStorage cache first
        let storage = Self::local_storage()?;
        let key = Self::cache_key(profile_id);
        if let Some(s) = storage.get_item(&key).map_err(|e| StorageError::Io(format!("get err: {:?}", e)))? {
            // Also refresh cache from IDB in the background
            Self::spawn_refresh_cache_from_idb(profile_id.to_string());
            return decode_state(&s);
        }
        // If not in cache, trigger an async fetch to populate cache next time and return NotFound
        Self::spawn_refresh_cache_from_idb(profile_id.to_string());
        Err(StorageError::NotFound(profile_id.to_string()))
    }

    fn save(&self, profile_id: &str, state: &core::GameState) -> Result<()> {
        let s = encode_state(state)?;
        // Update LocalStorage cache synchronously
        let storage = Self::local_storage()?;
        storage
            .set_item(&Self::cache_key(profile_id), &s)
            .map_err(|e| StorageError::Io(format!("set err: {:?}", e)))?;
        // Write to IndexedDB asynchronously
        Self::spawn_write_to_idb(profile_id.to_string(), s);
        Ok(())
    }
}

// Feature-selected storage alias and constructor helpers
#[cfg(feature = "desktop")]
pub type SelectedStorage = FsStorage;
#[cfg(feature = "wasm")]
pub type SelectedStorage = WasmIdbStorage;

/// Create the default storage for the currently selected platform feature.
pub fn new_default_storage() -> Result<SelectedStorage> {
    #[cfg(feature = "desktop")]
    {
        return FsStorage::new_default();
    }
    #[cfg(feature = "wasm")]
    {
        return Ok(WasmIdbStorage);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    #[cfg(feature = "desktop")]
    use tempfile::tempdir;

    // Helper arbitrary state aligned with core's ranges
    fn arb_state() -> impl Strategy<Value = core::GameState> {
        (
            any::<u64>(),
            -1_000i64..=1_000,
            -1_000i64..=1_000,
            -10_000i64..=10_000,
            -10_000i64..=10_000,
            -1_000_000i64..=1_000_000,
        ).prop_map(|(turn, pop_t, pop_e, pg, pc, rs)| core::GameState {
            turn,
            population_total: pop_t,
            population_employed: pop_e.clamp(0, pop_t.max(0)),
            power_generation: pg,
            power_consumption: pc,
            resources_stock: rs,
            layers_enabled: 0,
        })
    }

    #[cfg(feature = "desktop")]
    #[test]
    fn fs_storage_unit_roundtrip_and_listing() {
        let dir = tempdir().unwrap();
        let store = FsStorage::with_root_dir(dir.path()).unwrap();

        // initially empty
        assert!(store.list_profiles().unwrap().is_empty());

        let mut s = core::GameState::default();
        s.turn = 42;
        store.save("profile_a", &s).unwrap();
        store.save("profile_b", &s).unwrap();

        let mut profiles = store.list_profiles().unwrap();
        assert_eq!(profiles.len(), 2);
        assert_eq!(profiles[0].id, "profile_a");
        assert_eq!(profiles[1].id, "profile_b");

        let loaded = store.load("profile_a").unwrap();
        assert_eq!(loaded.turn, 42);

        // overwrite
        s.turn = 100;
        store.save("profile_a", &s).unwrap();
        let loaded2 = store.load("profile_a").unwrap();
        assert_eq!(loaded2.turn, 100);
    }

    #[cfg(feature = "desktop")]
    proptest! {
        #[test]
        fn fs_storage_property_roundtrip(state in arb_state()) {
            let dir = tempdir().unwrap();
            let store = FsStorage::with_root_dir(dir.path()).unwrap();
            store.save("prop", &state).unwrap();
            let loaded = store.load("prop").unwrap();
            prop_assert_eq!(loaded, state);
        }
    }

    #[test]
    fn selected_storage_type_compiles() {
        // This test ensures the SelectedStorage alias resolves under the active feature.
        // It doesn't perform IO on wasm.
        let _ = std::any::type_name::<SelectedStorage>();
    }
}
