use crate::syncers::rsyncer::{RsyncGlobalProperties, RsyncProperties};
use crate::Syncer;
use globset::{Glob, GlobSet, GlobSetBuilder};
use serde::{Deserialize, Deserializer};
use std::fmt::Debug;
use std::io::{Error as IOError, ErrorKind};
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::sync::Arc;

fn canonicalize<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    let local_dir = String::deserialize(deserializer)?;
    Path::new(&local_dir)
        .canonicalize()
        .or_else(|_e| {
            Err(serde::de::Error::invalid_value(
                serde::de::Unexpected::Str(&local_dir),
                &"path must exist",
            ))
        })?
        .into_os_string()
        .into_string()
        .map(|mut path| {
            if !path.ends_with("/") {
                path.push('/');
            }
            path
        })
        .or_else(|_e| {
            Err(serde::de::Error::invalid_value(
                serde::de::Unexpected::Str(&local_dir),
                &"Invalid path",
            ))
        })
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
enum WorkspaceSyncer {
    #[serde(rename(deserialize = "rsync"))]
    Rsync(RsyncProperties),
}

#[derive(Debug, Deserialize)]
struct WorkspaceConfig {
    #[serde(deserialize_with = "canonicalize")]
    src_dir: String,
    syncers: Vec<WorkspaceSyncer>,
    ignore: Option<Vec<String>>,
}

impl WorkspaceSyncer {
    fn as_syncer(&self, global_config: &GlobalConfig) -> Box<dyn Syncer> {
        match self {
            WorkspaceSyncer::Rsync(props) => {
                Box::new(props.as_syncer(global_config.rsync.as_ref()))
            }
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct Config {
    global_config: GlobalConfig,
    workspaces: Vec<WorkspaceConfig>,
}

#[derive(Debug, Deserialize)]
struct GlobalConfig {
    ignore: Option<Vec<String>>,
    rsync: Option<RsyncGlobalProperties>,
}

pub struct Workspace {
    pub path: String,
    // Not quite sure if Pin is necessary, but it just feels right
    pub syncers: Vec<Arc<Pin<Box<dyn Syncer>>>>,
    ignore: GlobSet,
}

impl Workspace {
    pub fn should_sync(&self, path: &Path) -> bool {
        !self.ignore.is_match(path)
    }
}

impl Config {
    pub async fn parse(path: &str) -> std::io::Result<Config> {
        let path = PathBuf::from(path);
        let data = tokio::fs::read(path).await?;
        serde_yaml::from_slice(&data).or_else(|e| Err(IOError::new(ErrorKind::InvalidData, e)))
    }

    pub fn workspaces(&self) -> Vec<Workspace> {
        self.workspaces
            .iter()
            .map(|ws_config| {
                let ws_path = Path::new(&ws_config.src_dir);
                let syncers = ws_config
                    .syncers
                    .iter()
                    .map(|properties| Arc::new(properties.as_syncer(&self.global_config).into()))
                    .collect();
                let mut builder = GlobSetBuilder::new();
                let mut add = |ignore: &Vec<String>| {
                    ignore.iter().for_each(|pattern| {
                        let path = Path::new(pattern);
                        if path.is_absolute() {
                            builder.add(Glob::new(pattern).unwrap());
                        } else {
                            let path = ws_path.join(path).into_os_string().into_string().unwrap();
                            builder.add(Glob::new(&path).unwrap());
                        }
                    })
                };
                self.global_config.ignore.as_ref().map(|i| add(i));
                ws_config.ignore.as_ref().map(|i| add(i));
                let ignores = builder.build().unwrap();
                Workspace {
                    path: ws_config.src_dir.clone(),
                    syncers,
                    ignore: ignores,
                }
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use crate::config::{Config, GlobalConfig, WorkspaceConfig};
    use std::path::Path;

    #[tokio::test]
    async fn it_works() {
        let config = Config::parse("examples/schema.config.yaml").await.unwrap();
        assert_eq!(config.workspaces.len(), 2);
    }

    #[test]
    fn workspaces_should_sync() {
        let config = Config {
            global_config: GlobalConfig {
                ignore: None,
                rsync: None,
            },
            workspaces: vec![WorkspaceConfig {
                src_dir: "/local/dir1".to_string(),
                syncers: vec![],
                ignore: Some(vec![
                    "*/ignore-1/*".to_string(),
                    "ignore-2/*".to_string(),
                    "ignore-3".to_string(),
                ]),
            }],
        };
        let workspaces = config.workspaces();
        let only_workspace = workspaces.get(0).unwrap();
        assert!(only_workspace.should_sync(Path::new("/local/dir1/random-file")));
        assert!(!only_workspace.should_sync(Path::new("/local/dir1/subdir1/ignore-1/file1")));
        assert!(!only_workspace.should_sync(Path::new("/local/dir1/subdir2/ignore-1/file2")));
        assert!(!only_workspace.should_sync(Path::new("/local/dir1/ignore-2/file2")));
        assert!(!only_workspace.should_sync(Path::new("/local/dir1/ignore-3")));
    }
}
