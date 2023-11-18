use clap::{command, Parser, Subcommand};

#[derive(Parser)]
#[command(about = "The blender version manager tool")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Test by reading a file and writing it back into another one
    Test {
        /// path of blender file to read
        #[arg(short, long)]
        from_path: String,
    },

    /// Create a checkpoint with the current contents of the file
    Commit {
        /// Path to the blend file DB
        #[arg(short, long)]
        db_path: String,

        /// Path to the blender file to create the commit from
        #[arg(short, long)]
        file_path: String,

        /// A short summary of the changes
        #[arg(short, long)]
        message: Option<String>,
    },

    /// Write the contents of a checkpoint to a file
    Restore {
        /// Path to the blend file DB
        #[arg(short, long)]
        db_path: String,

        /// Path of the file to write to
        #[arg(short, long)]
        file_path: String,

        /// The hash of the comit to check out
        #[arg(long)]
        hash: String,
    },

    /// Create a new branch
    NewBranch {
        /// Path to the blend file DB
        #[arg(short, long)]
        db_path: String,

        /// The name of the new branch
        #[arg(short, long)]
        branch_name: String,
    },

    /// Delete a brach
    DeleteBranch {
        /// Path to the blend file DB
        #[arg(short, long)]
        db_path: String,

        /// The name of the new branch
        #[arg(short, long)]
        branch_name: String,
    },

    /// Lists all existing branches
    ListBranches {
        /// Path to the blend file DB
        #[arg(short, long)]
        db_path: String,
    },

    /// Switch to the latest version on a branch
    Switch {
        /// Path to the blend file DB
        #[arg(short, long)]
        db_path: String,

        /// Name of the branch to switch to
        #[arg(short, long)]
        branch: String,

        /// Path of the file to write to
        #[arg(short, long)]
        file_path: String,
    },

    // Gets the currently active branch
    GetCurrentBranch {
        /// Path to the blend file DB
        #[arg(short, long)]
        db_path: String,
    },

    /// Log all checkpoints for the current branch or a specified branch
    LogCheckpoints {
        /// Path to the blend file DB
        #[arg(short, long)]
        db_path: String,

        /// name of the branch to log checkpoints for
        #[arg(short, long)]
        branch: String,
    },

    /// Initialize the DB
    Init {
        /// Path to the blend file DB
        #[arg(short, long)]
        db_path: String,

        /// Path to the blender file to create the DB from
        #[arg(short, long)]
        file_path: String,
    },

    InitFromImport {
        /// Url of the external server
        #[arg(short, long)]
        url: String,

        /// Path to the blend file DB
        #[arg(short, long)]
        db_path: String,

        /// Path to the blender file to create the DB from
        #[arg(short, long)]
        file_path: String,

        /// ID of project to import
        #[arg(short, long)]
        project_id: String,
    },

    /// Export descendants of a commit
    Export {
        /// Path to the blend file DB
        #[arg(short, long)]
        db_path: String,

        /// Export descendants of this commit
        #[arg(short, long)]
        from_commit: String,

        /// Path to the exchange file
        #[arg(short, long)]
        path_to_exchange: String,
    },

    /// Import an exchange file
    Import {
        /// Path to the blend file DB
        #[arg(short, long)]
        db_path: String,

        /// Path to the exchange file to be imported
        #[arg(short, long)]
        path_to_exchange: String,
    },

    /// Sync to an external server
    SyncToServer {
        /// Path to the blend file DB
        #[arg(short, long)]
        db_path: String,

        /// Url of the external server
        #[arg(short, long)]
        url: String,
    },
}

pub fn parse_args() -> Cli {
    Cli::parse()
}
