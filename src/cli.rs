//! Argument parsing. Kept separate from [`crate::run`] so the seam stays a
//! plain function over argv.

use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(
    name = "agent-sync",
    version,
    about = "One canonical .agents/ folder, fanned out to all your AI coding agents",
    long_about = "One canonical .agents/ folder, fanned out to all your AI coding agents.\n\n\
                  Sync skills, instructions, MCP servers, commands, agents and hooks \
                  from the Commons — the canonical ~/.agents directory — to every installed agent.",
    after_help = "Home: https://agent-sync.sh"
)]
pub struct Cli {
    /// Absent means `sync`: it is the command users run every day, and typing
    /// `agent-sync sync` to get it reads as a stutter.
    #[command(subcommand)]
    pub command: Option<Command>,
}

/// Everything one `sync` invocation can narrow. Empty vectors and tables mean
/// the whole machine, exactly as before this struct existed.
#[derive(clap::Args, Debug, Default)]
pub struct SyncArgs {
    /// Report the plan without touching the filesystem.
    #[arg(long)]
    pub dry_run: bool,
    /// Sync only these Targets. Repeat the flag for more than one.
    #[arg(long = "agent", value_name = "NAME")]
    pub agent: Vec<String>,
    /// Use an alternate config root for the one selected agent. Repeat for
    /// multiple profiles of that agent.
    #[arg(long = "root", value_name = "PATH")]
    pub root: Vec<String>,
    /// Sync only a surface, or named items inside it:
    /// skills[:ITEM[,ITEM...]], commands, agents, instructions, mcp, hooks.
    /// Family and item names accept any unique, case-insensitive prefix.
    #[arg(long = "only", value_name = "FAMILY[:ITEM[,ITEM...]]")]
    pub only: Vec<String>,
    /// Survey every Target and surface before writing, not just the selection.
    #[arg(long)]
    pub strict: bool,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    /// Fan the Commons out to every installed agent.
    Sync(SyncArgs),
    /// Create the Commons skeleton and report what this machine already has.
    Init,
    /// Take a path under management; where it lives picks the mechanic.
    ///
    /// Three mechanics, one verb. A real config in a Target surface is
    /// absorbed into the Commons, leaving a link behind. A path inside a git
    /// repo becomes a Sourced entry — the Commons links out to it, and the
    /// repo keeps the truth. Any other path is copied in, and the original is
    /// no longer consulted. A path already inside the Commons refuses: it is
    /// already home.
    Adopt {
        /// A Target-surface path to absorb, a repo path to Source, or any
        /// other path to copy in.
        path: String,
        /// Name the mechanic and report every action without touching anything.
        #[arg(long)]
        dry_run: bool,
    },
    /// Remove everything agent-sync put into one target.
    Revert {
        /// The agent to strip, by the name `status` and `doctor` use.
        agent: String,
    },
    /// Report what is in sync, what is not, and what is not ours.
    Status {
        /// Emit a machine-readable report on stdout.
        #[arg(long)]
        json: bool,
    },
    /// Manage MCP servers.
    Mcp {
        #[command(subcommand)]
        command: McpCommand,
    },
    /// Check machine readiness: installed agents, Commons hygiene, Sourced entries.
    Doctor,
}

#[derive(Subcommand, Debug)]
pub enum McpCommand {
    /// Show Commons servers and their state in every agent.
    List {
        /// Emit a machine-readable report on stdout.
        #[arg(long)]
        json: bool,
    },
    /// Take an agent's existing server into the Commons.
    Adopt {
        /// Which server, as agent/server — for example claude/serena.
        spec: Option<String>,
        /// Adopt every server found in every agent's config.
        #[arg(long, conflicts_with = "spec")]
        all: bool,
    },
    /// Delete a server from the Commons and from every agent.
    Remove {
        /// The server name.
        name: String,
    },
    /// Park a server: keep its definition, render it nowhere.
    Disable {
        /// The server name.
        name: String,
    },
    /// Restore a disabled server to every targeted agent.
    Enable {
        /// The server name.
        name: String,
    },
}
