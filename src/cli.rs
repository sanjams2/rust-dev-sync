use clap::Parser;
use crate::Syncer;

#[derive(clap::Subcommand, Debug)]
pub enum WorkspaceSyncerCommand {
    /// Add a syncer to a workspace
    Add{
        #[clap(short = 't', long = "--type", arg_enum)]
        syncer_type: SyncerType,
        // TODO:
        //#[clap(short, long, parse(from_str = props_from_str))]
        //properties: Box<dyn Syncer>,
    },
    /// Remove a certain syncer from a workspace
    Remove{
        #[clap(short = 't', long = "--type", arg_enum)]
        syncer_type: SyncerType,
    },
}

#[derive(Debug, clap::ArgEnum, Clone)]
pub enum SyncerType {
    /// Rsync syncer type
    Rsync,
}

#[derive(clap::Subcommand, Debug)]
pub enum WorkspaceCommand {
    /// Add a new workspace to the config file
    Add,
    /// Remove a workspace from the config file
    Remove,
    /// Perform operations on workspace syncers
    #[clap(subcommand)]
    Syncers(WorkspaceSyncerCommand),
}

#[derive(clap::Subcommand, Debug)]
pub enum Command {
    /// Continually sync workspaces
    Sync,
    /// Initialize config file
    Init,
    /// Operation on workspaces
    Workspace{
        #[clap(subcommand)]
        command: WorkspaceCommand,
        #[clap(short, long)]
        src_path: String,
    },
}

#[derive(Parser)]
#[clap(author, version, about, long_about = None)]
pub struct Cli {
    /// Path of the config file specifying workspace to sync and syncers to use
    #[clap(short, long)]
    pub config: Option<String>,
    #[clap(subcommand)]
    pub command: Command,
}

impl Cli {

    pub fn parse() -> Self {
        Parser::parse()
    }

}