use serde::{Deserialize, Serialize};
use strum_macros::EnumString;

#[derive(Debug, PartialEq, Clone, Deserialize, Serialize, EnumString)]
pub enum RsyncFlag {
    Recursive,
    IncludeLinks,
    PreservePermissions,
    PreserveModTimes,
    Compress,
    Verbose,
    DeleteAfter,
}

impl RsyncFlag {
    pub fn as_cli_arg(&self) -> &str {
        match self {
            RsyncFlag::Recursive => "-r",
            RsyncFlag::IncludeLinks => "-l",
            RsyncFlag::PreservePermissions => "-p",
            RsyncFlag::PreserveModTimes => "-t",
            RsyncFlag::Compress => "-z",
            RsyncFlag::Verbose => "-v",
            RsyncFlag::DeleteAfter => "--delete-after",
        }
    }
}

#[derive(Debug, PartialEq, Clone, Deserialize, Serialize, EnumString)]
pub enum RsyncOption {
    Exclude(String),
}

impl RsyncOption {
    pub fn as_cli_args(&self) -> Vec<String> {
        let (name, value) = match self {
            RsyncOption::Exclude(x) => (String::from("--exclude"), x.clone()),
        };
        vec![name, value]
    }
}
