use serde::{Deserialize, Serialize};
use strum_macros::EnumString;

#[derive(Debug, PartialEq, Clone, Deserialize, Serialize, EnumString)]
pub enum SSHOption {
    PasswordAuthentication(bool),
    ServerAliveInterval(i32),
    ServerAliveCountMax(i32),
    ConnectTimeout(i32),
    ControlMaster(String),
    ControlPersist(String),
    ControlPath(String),
    IdentityFile(String),
}

impl SSHOption {
    fn as_cli_opt(&self) -> String {
        match self {
            SSHOption::PasswordAuthentication(v) => {
                format!("-o PasswordAuthentication={}", v.to_string())
            }
            SSHOption::ServerAliveInterval(v) => {
                format!("-o ServerAliveInterval={}", v.to_string())
            }
            SSHOption::ServerAliveCountMax(v) => {
                format!("-o ServerAliveCountMax={}", v.to_string())
            }
            SSHOption::ConnectTimeout(v) => format!("-o ConnectTimeout={}", v.to_string()),
            SSHOption::ControlMaster(v) => format!("-o ControlMaster={}", v.to_string()),
            SSHOption::ControlPersist(v) => format!("-o ControlPersist={}", v.to_string()),
            SSHOption::ControlPath(v) => format!("-o ControlPath={}", v.to_string()),
            SSHOption::IdentityFile(v) => format!("-i {}", v.to_string()),
        }
    }
}

impl Drop for SSHOption {
    fn drop(&mut self) {
        // Not sure if performing I/O in the drop method is a great idea, but
        // this is serving as a good way for me to test and experiment with drop
        if let SSHOption::ControlPath(x) = self {
            if let Err(e) = std::fs::remove_file(x) {
                if e.kind() != std::io::ErrorKind::NotFound {
                    println!("Error closing ssh session control path: {:?}", e);
                }
            }
        }
    }
}

#[derive(Debug)]
pub struct SSHShell {
    pub options: Vec<SSHOption>,
}

impl SSHShell {
    pub fn new(options: Vec<SSHOption>) -> Self {
        SSHShell { options }
    }
}

impl SSHShell {
    pub fn as_arg(&self) -> String {
        let opts = self
            .options
            .iter()
            .map(|opt| opt.as_cli_opt())
            .collect::<Vec<String>>()
            .join(" -o ");
        format!("ssh -o {}", opts)
    }
}

#[cfg(test)]
mod tests {
    use crate::rsync::shell::ssh::SSHOption;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn control_path_option_cleans_up_session_file_when_dropped() {
        let epoch_millis = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis();
        let path = format!("/tmp/rust-sync-control-path-test-{}", epoch_millis);
        let path_clone_1 = path.clone();
        let path_clone_2 = path.clone();
        if let Err(e) = std::fs::OpenOptions::new()
            .create(true)
            .write(true)
            .open(path)
        {
            panic!("Error creating path for test: {:?}", e);
        }

        {
            let cp = SSHOption::ControlPath(path_clone_1);
            println!("CP: {:?}", cp);
        }

        match std::fs::metadata(path_clone_2) {
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => panic!("Error stating control path file: {:?}", e),
            Ok(_) => panic!("File should not exist after SSHOption::ControlPath was dropped"),
        }
    }
}
