pub mod cli;
pub mod shell;

use crate::rsync::cli::{RsyncFlag, RsyncOption};
use crate::rsync::shell::ssh::SSHShell;
use tokio::process::Command;

pub type Result = std::result::Result<(), String>;

fn command(
    src: &str,
    dst: &str,
    dst_host: Option<&str>,
    dst_host_user: Option<&str>,
    shell: Option<&SSHShell>,
    flags: &[RsyncFlag],
    options: &[RsyncOption],
) -> Command {
    let mut cmd = Command::new("rsync");
    // Add rsync options
    for flag in flags {
        cmd.arg(flag.as_cli_arg());
    }
    if shell.is_some() {
        cmd.arg("-e");
        cmd.arg(shell.as_ref().unwrap().as_arg());
    }
    for opt in options {
        cmd.args(opt.as_cli_args());
    }
    cmd.arg(src);
    let host = vec![dst_host_user, dst_host]
        .into_iter()
        .flatten()
        .collect::<Vec<&str>>()
        .join("@");
    if host.is_empty() {
        cmd.arg(dst);
    } else {
        cmd.arg(format!("{}:{}", host, dst));
    }
    cmd
}

// TODO: the result here should not be a syncer result
pub async fn rsync(
    src: &str,
    dst: &str,
    dst_host: Option<&str>,
    dst_host_usr: Option<&str>,
    shell: Option<&SSHShell>,
    flags: &[RsyncFlag],
    options: &[RsyncOption],
) -> Result {
    let mut cmd = command(src, dst, dst_host, dst_host_usr, shell, flags, options);
    println!("Running: '{:?}'", cmd);
    match cmd.output().await {
        Err(e) => Err(format!("Error running command: {:?}", e)),
        Ok(output) => {
            if !output.status.success() {
                let stderr = String::from_utf8(output.stderr)
                    .or::<std::result::Result<String, ()>>(Ok("unable to decode stderr".to_owned()))
                    .unwrap();
                return Err(format!(
                    "Error Status: {}, StdErr:\n{}",
                    output.status, stderr
                ));
            }
            if !output.stderr.is_empty() {
                println!("---------- rsync stderr ----------");
                print!("{}", String::from_utf8(output.stderr).unwrap());
                println!("----------------------------------");
            }
            if !output.stdout.is_empty() {
                println!("---------- rsync stdout ----------");
                print!("{}", String::from_utf8(output.stdout).unwrap());
                println!("----------------------------------");
            }
            Ok(())
        }
    }
}
