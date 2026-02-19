use anyhow::Context;
use camino::{Utf8Path, Utf8PathBuf};
use depguard_domain::model::{ManifestModel, WorkspaceDependency};
use depguard_types::RepoPath;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::time::SystemTime;

pub const MANIFEST_CACHE_FILENAME: &str = "manifests.v1.json";
const MANIFEST_CACHE_VERSION: u32 = 1;

#[derive(Clone, Debug, Default)]
pub struct ManifestCache {
    path: Utf8PathBuf,
    file: ManifestCacheFile,
    dirty: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct ManifestCacheFile {
    version: u32,
    entries: BTreeMap<String, ManifestCacheEntry>,
}

impl Default for ManifestCacheFile {
    fn default() -> Self {
        Self {
            version: MANIFEST_CACHE_VERSION,
            entries: BTreeMap::new(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct ManifestCacheEntry {
    stamp: ManifestStamp,
    manifest: ManifestModel,
    workspace_dependencies: Option<BTreeMap<String, WorkspaceDependency>>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ManifestStamp {
    len: u64,
    modified_secs: u64,
    modified_nanos: u32,
}

impl ManifestStamp {
    pub fn from_path(path: &Utf8Path) -> anyhow::Result<Self> {
        let meta = std::fs::metadata(path).with_context(|| format!("read metadata {}", path))?;
        let modified = meta.modified().unwrap_or(SystemTime::UNIX_EPOCH);
        let since_epoch = modified
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default();
        Ok(Self {
            len: meta.len(),
            modified_secs: since_epoch.as_secs(),
            modified_nanos: since_epoch.subsec_nanos(),
        })
    }
}

impl ManifestCache {
    pub fn load(repo_root: &Utf8Path, cache_dir: &Utf8Path) -> anyhow::Result<Self> {
        let abs_cache_dir = if cache_dir.is_absolute() {
            cache_dir.to_path_buf()
        } else {
            repo_root.join(cache_dir)
        };
        let cache_path = abs_cache_dir.join(MANIFEST_CACHE_FILENAME);

        let file = match std::fs::read_to_string(&cache_path) {
            Ok(text) => match serde_json::from_str::<ManifestCacheFile>(&text) {
                Ok(parsed) if parsed.version == MANIFEST_CACHE_VERSION => parsed,
                _ => ManifestCacheFile::default(),
            },
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => ManifestCacheFile::default(),
            Err(err) => {
                return Err(err).with_context(|| format!("read manifest cache {}", cache_path));
            }
        };

        Ok(Self {
            path: cache_path,
            file,
            dirty: false,
        })
    }

    pub fn root_if_fresh(
        &self,
        manifest: &RepoPath,
        stamp: ManifestStamp,
    ) -> Option<(BTreeMap<String, WorkspaceDependency>, ManifestModel)> {
        let entry = self.file.entries.get(manifest.as_str())?;
        if entry.stamp != stamp {
            return None;
        }
        let workspace_dependencies = entry.workspace_dependencies.clone()?;
        Some((workspace_dependencies, entry.manifest.clone()))
    }

    pub fn member_if_fresh(
        &self,
        manifest: &RepoPath,
        stamp: ManifestStamp,
    ) -> Option<ManifestModel> {
        let entry = self.file.entries.get(manifest.as_str())?;
        if entry.stamp != stamp {
            return None;
        }
        Some(entry.manifest.clone())
    }

    pub fn store_root(
        &mut self,
        manifest: &RepoPath,
        stamp: ManifestStamp,
        workspace_dependencies: &BTreeMap<String, WorkspaceDependency>,
        model: &ManifestModel,
    ) {
        self.file.entries.insert(
            manifest.as_str().to_string(),
            ManifestCacheEntry {
                stamp,
                manifest: model.clone(),
                workspace_dependencies: Some(workspace_dependencies.clone()),
            },
        );
        self.dirty = true;
    }

    pub fn store_member(
        &mut self,
        manifest: &RepoPath,
        stamp: ManifestStamp,
        model: &ManifestModel,
    ) {
        self.file.entries.insert(
            manifest.as_str().to_string(),
            ManifestCacheEntry {
                stamp,
                manifest: model.clone(),
                workspace_dependencies: None,
            },
        );
        self.dirty = true;
    }

    pub fn save_if_dirty(&mut self) -> anyhow::Result<()> {
        if !self.dirty {
            return Ok(());
        }
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent).with_context(|| format!("create {}", parent))?;
        }
        let json = serde_json::to_string_pretty(&self.file).context("serialize manifest cache")?;
        std::fs::write(&self.path, json).with_context(|| format!("write {}", self.path))?;
        self.dirty = false;
        Ok(())
    }
}
