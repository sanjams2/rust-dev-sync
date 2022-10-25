mod fstree;
mod notify_tokio;
mod rsync;
mod syncers;
mod config;

use std::path::PathBuf;
use notify::{RecursiveMode, Watcher};

use crate::syncers::Syncer;

#[tokio::main]
async fn main() {
    let home_dir = home::home_dir().unwrap().into_os_string().into_string().unwrap();
    let config_path = home_dir + "~/.config/rust-dev-sync-config.yaml";
    println!("Using config at location: {}", config_path);
    let config = config::Config::parse(&config_path).await.unwrap();

    let mut workspace_tree = fstree::FsTree::new();
    for workspace in config.workspaces() {
        workspace_tree.insert(&PathBuf::from(&workspace.path), workspace);
    }

    let (handler, mut receiver) = notify_tokio::TokioEventHandler::unbounded();
    let mut watcher = notify::recommended_watcher(handler).unwrap();

    for workspace in &workspace_tree {
        println!("Monitoring workspace: {:?}", workspace.path);
        watcher
            .watch(workspace.path.as_ref(), RecursiveMode::Recursive)
            .unwrap();
    }

    tokio::spawn(async move {
        if let Err(e) = tokio::signal::ctrl_c().await {
            println!("Error awaiting control-c action: {:?}", e)
        }
        // Needed to close the Sender which will signal to the receiver that there is nothing left
        drop(watcher);
    });

    while let Some(event) = receiver.recv().await {
        match event {
            Ok(event) => {
                println!("Received event: {:?}", event);
                let path = event.paths.get(0).unwrap();
                if let Some(workspace) = workspace_tree.get_closest(path) {
                    if workspace.should_sync(path.as_path()) {
                        for syncer in &workspace.syncers {
                            let thread_syncer = syncer.clone();
                            let workspace_path = workspace.path.clone();
                            let file_path = path.clone();
                            let kind = event.kind.clone();
                            tokio::spawn(async move {
                                if let Err(e) = thread_syncer
                                    .sync(workspace_path.as_ref(), file_path.as_path(), kind)
                                    .await
                                {
                                    println!("Error during sync: {}", e);
                                }
                            });
                        }
                    }
                }
            }
            Err(e) => println!("Received error event: {:?}", e),
        }
    }
    println!("Exiting...");
}
