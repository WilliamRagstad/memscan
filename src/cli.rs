use clap::{Parser, Subcommand, ValueHint, builder::styling::AnsiColor};

/// MemScan – inspect another process's virtual memory.
#[derive(Parser, Debug)]
#[command(
    name = "memscan",
	bin_name = "memscan",
    about = "A simple Windows process memory scanner",
    version,
    propagate_version = true,
    arg_required_else_help = true,
	styles = clap::builder::Styles::styled()
		.header(AnsiColor::BrightYellow.on_default())
        .usage(AnsiColor::BrightYellow.on_default())
        .literal(AnsiColor::BrightGreen.on_default())
        .placeholder(AnsiColor::BrightCyan.on_default())
)]
pub struct Cli {
    /// Increase verbosity (-v, -vv, -vvv)
    #[arg(short, long, global = true, action = clap::ArgAction::Count)]
    pub verbose: u8,

    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    /// Scan a process's memory regions
    Scan {
        /// Target process executable name or id (e.g. "notepad", "notepad.exe", or 1234)
        target: String,

        /// Optional hex pattern to search for (e.g. "DEADBEEF")
        #[arg(short, long, value_hint = ValueHint::Other)]
        pattern: Option<String>,
    },
}
