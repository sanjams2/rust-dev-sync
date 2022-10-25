use std::fmt::Debug;
use std::path::Path;

use async_trait::async_trait;
use notify::EventKind;
use serde::Deserialize;

use rand;
use rand::distributions::Alphanumeric;
use rand::Rng;

use crate::rsync;
use crate::rsync::cli::{RsyncFlag, RsyncOption};
use crate::rsync::shell::ssh::{SSHOption, SSHShell};
use crate::syncers::{Result as SyncerResult, Syncer};

#[derive(Debug)]
pub struct Rsyncer {
    dst_dir: String,
    dst_host: Option<String>,
    flags: Vec<RsyncFlag>,
    options: Vec<RsyncOption>,
    shell: Option<SSHShell>,
}

impl Rsyncer {
    pub fn new(
        dst_dir: &str,
        dst_host: Option<&str>,
        flags: &[RsyncFlag],
        options: &[RsyncOption],
        shell: Option<SSHShell>,
    ) -> Self {
        Rsyncer {
            dst_dir: String::from(dst_dir),
            dst_host: dst_host.map(String::from),
            flags: Vec::from(flags),
            options: Vec::from(options),
            shell: shell,
        }
    }
}

#[async_trait]
impl Syncer for Rsyncer {
    async fn sync(&self, workspace_path: &Path, _file_path: &Path, _kind: EventKind) -> SyncerResult {
        rsync::rsync(
            workspace_path.to_str().unwrap(),
            self.dst_dir.as_ref(),
            self.dst_host.as_deref(),
            None,
            self.shell.as_ref(),
            self.flags.as_slice(),
            self.options.as_slice(),
        )
        .await
    }
}

#[derive(Debug, PartialEq, Clone, Deserialize)]
pub struct SSHProperties {
    options: Option<Vec<SSHOption>>,
}

impl SSHProperties {
    pub fn merge(&self, additional_props: Option<&SSHAdditionalProperties>) -> Self {
        if additional_props.is_none() {
            return self.clone();
        }
        let additional_props = additional_props.unwrap();
        let options = self
            .options
            .as_ref()
            .map(|opts| {
                opts.iter()
                    .chain(additional_props.additional_options.iter().flatten())
                    .cloned()
                    .collect::<Vec<SSHOption>>()
            })
            .or(additional_props.additional_options.clone());
        SSHProperties { options }
    }

    fn as_shell(&self) -> SSHShell {
        let options = self
            .options
            .as_ref()
            .map(|opts| {
                opts.iter()
                    .map(|opt| match opt {
                        SSHOption::ControlPath(ref path) if path == "GENERATE" => {
                            let session_id = generate_session_id();
                            let cp = generate_control_path(&session_id);
                            SSHOption::ControlPath(cp)
                        }
                        _ => opt.clone(),
                    })
                    .collect()
            })
            .unwrap_or(Vec::new());
        SSHShell::new(options)
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct RsyncProperties {
    dst_host: Option<String>,
    dst_dir: String,
    additional_flags: Option<Vec<RsyncFlag>>,
    additional_excludes: Option<Vec<String>>,
    ssh: Option<SSHAdditionalProperties>,
}

#[derive(Debug, Deserialize)]
pub struct RsyncGlobalProperties {
    default_dst_host: Option<String>,
    excludes: Option<Vec<String>>,
    flags: Option<Vec<RsyncFlag>>,
    ssh: Option<SSHProperties>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct SSHAdditionalProperties {
    additional_options: Option<Vec<SSHOption>>,
}

impl RsyncProperties {
    pub fn as_syncer(&self, global_props: Option<&RsyncGlobalProperties>) -> Rsyncer {
        let excludes = self
            .additional_excludes
            .iter()
            .flatten()
            .chain(
                global_props
                    .map(|prop| prop.excludes.as_ref())
                    .flatten()
                    .iter()
                    .cloned()
                    .flatten(),
            )
            .cloned()
            .map(RsyncOption::Exclude)
            .collect::<Vec<RsyncOption>>();
        let flags = self
            .additional_flags
            .iter()
            .flatten()
            .chain(
                global_props
                    .map(|prop| prop.flags.as_ref())
                    .flatten()
                    .iter()
                    .cloned()
                    .flatten(),
            )
            .cloned()
            .collect::<Vec<RsyncFlag>>();
        let dst_host = self.dst_host.clone().or_else(|| {
            global_props
                .map(|prop| prop.default_dst_host.as_ref())
                .flatten()
                .cloned()
        });
        let shell = global_props
            .map(|props| props.ssh.as_ref())
            .flatten()
            .as_deref()
            .or_else(|| self.ssh.as_ref().map(|_| &SSHProperties { options: None }))
            .map(|props| props.merge(self.ssh.as_ref()))
            .map(|props| props.as_shell());
        Rsyncer::new(&self.dst_dir, dst_host.as_deref(), &flags, &excludes, shell)
    }
}

fn generate_control_path(session_id: &str) -> String {
    let mut control_path = home::home_dir().unwrap();
    control_path.push(".ssh");
    control_path.push("rust-dev-sync-".to_owned() + &session_id);
    control_path.into_os_string().into_string().unwrap()
}

fn generate_session_id() -> String {
    rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(7)
        .map(char::from)
        .collect()
}

#[cfg(test)]
mod tests {
    use crate::rsync::cli::{RsyncFlag, RsyncOption};
    use crate::rsync::shell::ssh::SSHOption;
    use crate::syncers::rsyncer::{
        RsyncGlobalProperties, RsyncProperties, SSHAdditionalProperties, SSHProperties,
    };

    #[test]
    fn test_sshproperties_merge() {
        let props = SSHProperties {
            options: Some(vec![
                SSHOption::ConnectTimeout(20),
                SSHOption::IdentityFile(".ssh/secret-pem".to_string()),
            ]),
        };
        let additional_props = SSHAdditionalProperties {
            additional_options: Some(vec![
                SSHOption::ServerAliveInterval(10),
                SSHOption::ConnectTimeout(30),
            ]),
        };
        let final_props = props.merge(Some(&additional_props));
        let final_props_options = final_props.options.as_ref().unwrap();
        assert!(final_props_options.contains(&SSHOption::ConnectTimeout(30)));
        assert!(
            final_props_options.contains(&SSHOption::IdentityFile(".ssh/secret-pem".to_string()))
        );
        assert!(final_props_options.contains(&SSHOption::ServerAliveInterval(10)))
    }

    #[test]
    fn test_sshproperties_merge_with_no_options() {
        let props = SSHProperties { options: None };
        let additional_props = SSHAdditionalProperties {
            additional_options: None,
        };
        let final_props = props.merge(Some(&additional_props));
        assert_eq!(final_props.options, None);
    }

    #[test]
    fn test_sshproperties_as_shell() {
        let props = SSHProperties {
            options: Some(vec![
                SSHOption::ConnectTimeout(20),
                SSHOption::IdentityFile(".ssh/secret-pem".to_string()),
                SSHOption::ControlPath("GENERATE".to_string()),
            ]),
        };
        let shell = props.as_shell();
        let cp = shell.options.iter().find(|opt| match opt {
            SSHOption::ControlPath(x) => x.contains(".ssh/rust-dev-sync-"),
            _ => false,
        });
        assert!(
            cp.is_some(),
            "SSHShell should have a control-path option with a properly generated path"
        );
    }

    #[test]
    fn test_rsyncproperties_as_syncer() {
        let props = RsyncProperties {
            dst_host: Some("override-host".to_string()),
            dst_dir: "/remote/dir".to_string(),
            additional_flags: Some(vec![RsyncFlag::IncludeLinks]),
            additional_excludes: Some(vec!["additional-exclude-1".to_string()]),
            ssh: Some(SSHAdditionalProperties {
                additional_options: Some(vec![
                    SSHOption::ConnectTimeout(5),
                    SSHOption::PasswordAuthentication(false),
                ]),
            }),
        };
        let global_props = RsyncGlobalProperties {
            default_dst_host: Some("default-host".to_string()),
            excludes: Some(vec![
                "global-exclude-1".to_string(),
                "global-exclude-2".to_string(),
            ]),
            flags: Some(vec![RsyncFlag::Recursive, RsyncFlag::DeleteAfter]),
            ssh: Some(SSHProperties {
                options: Some(vec![
                    SSHOption::ConnectTimeout(3),
                    SSHOption::ServerAliveCountMax(12),
                ]),
            }),
        };
        let rsyncer = props.as_syncer(Some(&global_props));
        let rsyncer_options = rsyncer.options;
        assert!(rsyncer_options.contains(&RsyncOption::Exclude("additional-exclude-1".to_string())));
        assert!(rsyncer_options.contains(&RsyncOption::Exclude("global-exclude-1".to_string())));
        assert!(rsyncer_options.contains(&RsyncOption::Exclude("global-exclude-2".to_string())));
        let rsyncer_flags = rsyncer.flags;
        assert!(rsyncer_flags.contains(&RsyncFlag::Recursive));
        assert!(rsyncer_flags.contains(&RsyncFlag::DeleteAfter));
        assert!(rsyncer_flags.contains(&RsyncFlag::IncludeLinks));
        assert_eq!(rsyncer.dst_host.unwrap(), "override-host".to_string());
        assert_eq!(rsyncer.dst_dir, "/remote/dir");
        let rsyncer_ssh = rsyncer.shell.as_ref().unwrap();
        assert!(rsyncer_ssh.options.contains(&SSHOption::ConnectTimeout(5)));
        assert!(rsyncer_ssh
            .options
            .contains(&SSHOption::PasswordAuthentication(false)));
        assert!(rsyncer_ssh
            .options
            .contains(&SSHOption::ServerAliveCountMax(12)));
    }

    #[test]
    fn test_rsyncproperties_as_syncer_when_no_defaults() {
        let props = RsyncProperties {
            dst_host: Some("override-host".to_string()),
            dst_dir: "/remote/dir".to_string(),
            additional_flags: Some(vec![RsyncFlag::IncludeLinks]),
            additional_excludes: Some(vec!["additional-exclude-1".to_string()]),
            ssh: Some(SSHAdditionalProperties {
                additional_options: Some(vec![
                    SSHOption::ConnectTimeout(5),
                    SSHOption::PasswordAuthentication(false),
                ]),
            }),
        };
        let global_props = RsyncGlobalProperties {
            default_dst_host: None,
            excludes: None,
            flags: None,
            ssh: None,
        };
        let rsyncer = props.as_syncer(Some(&global_props));
        let rsyncer_options = rsyncer.options;
        assert!(rsyncer_options.contains(&RsyncOption::Exclude("additional-exclude-1".to_string())));
        let rsyncer_flags = rsyncer.flags;
        assert!(rsyncer_flags.contains(&RsyncFlag::IncludeLinks));
        assert_eq!(rsyncer.dst_host.unwrap(), "override-host".to_string());
        assert_eq!(rsyncer.dst_dir, "/remote/dir");
        let rsyncer_ssh = rsyncer.shell.as_ref().unwrap();
        assert!(rsyncer_ssh.options.contains(&SSHOption::ConnectTimeout(5)));
        assert!(rsyncer_ssh
            .options
            .contains(&SSHOption::PasswordAuthentication(false)));
    }

    #[test]
    fn test_rsyncproperties_as_syncer_when_no_overrides() {
        let props = RsyncProperties {
            dst_host: None,
            dst_dir: "/remote/dir".to_string(),
            additional_flags: None,
            additional_excludes: None,
            ssh: None,
        };
        let global_props = RsyncGlobalProperties {
            default_dst_host: Some("default-host".to_string()),
            excludes: Some(vec![
                "global-exclude-1".to_string(),
                "global-exclude-2".to_string(),
            ]),
            flags: Some(vec![RsyncFlag::Recursive, RsyncFlag::DeleteAfter]),
            ssh: Some(SSHProperties {
                options: Some(vec![
                    SSHOption::ConnectTimeout(3),
                    SSHOption::ServerAliveCountMax(12),
                ]),
            }),
        };
        let rsyncer = props.as_syncer(Some(&global_props));
        let rsyncer_options = rsyncer.options;
        assert!(rsyncer_options.contains(&RsyncOption::Exclude("global-exclude-1".to_string())));
        assert!(rsyncer_options.contains(&RsyncOption::Exclude("global-exclude-2".to_string())));
        let rsyncer_flags = rsyncer.flags;
        assert!(rsyncer_flags.contains(&RsyncFlag::Recursive));
        assert!(rsyncer_flags.contains(&RsyncFlag::DeleteAfter));
        assert_eq!(rsyncer.dst_host.unwrap(), "default-host".to_string());
        assert_eq!(rsyncer.dst_dir, "/remote/dir");
        let rsyncer_ssh = rsyncer.shell.as_ref().unwrap();
        assert!(rsyncer_ssh.options.contains(&SSHOption::ConnectTimeout(3)));
        assert!(rsyncer_ssh
            .options
            .contains(&SSHOption::ServerAliveCountMax(12)));
    }
}
