use clap::{Parser, Subcommand, ValueHint, builder::styling::AnsiColor};
use libmemscan::{
    parse_hex_pattern,
    process::{find_process_by_name, get_process_module_regions, open_process, query_system_info},
    scanner::{ScanOptions, scan_process},
};
use owo_colors::OwoColorize;

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

        /// Scan all modules, including those not originating from the target process
        /// (by default, only the process's own modules are scanned)
        #[arg(long)]
        all_modules: bool,

        /// Disable memory mapping and use ReadProcessMemory instead
        /// (memory mapping is enabled by default for better performance)
        #[arg(long)]
        no_memmap: bool,
    },
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Command::Scan {
            target,
            pattern,
            all_modules,
            no_memmap,
        } => {
            let pid = if target.chars().all(|c| c.is_ascii_digit()) {
                let pid: u32 = target.parse()?;
                println!("{} target pid={}", "[info]".bright_cyan(), pid);
                pid
            } else {
                println!(
                    "{} looking up process by name: {}",
                    "[info]".bright_cyan(),
                    target
                );
                let pid = find_process_by_name(&target)?
                    .ok_or_else(|| anyhow::anyhow!("process with name '{}' not found", target))?;
                println!("{} found pid={}", "[info]".bright_cyan(), pid);
                pid
            };
            let proc = open_process(pid)?;

            let sys = query_system_info();
            println!(
                "{} system info: min_addr={:016x}, max_addr={:016x}, page_size={}, granularity={}",
                "[info]".bright_cyan(),
                sys.min_app_addr,
                sys.max_app_addr,
                sys.page_size,
                sys.granularity
            );

            let modules = get_process_module_regions(&proc)?;
            println!(
                "{} found {} module regions",
                "[info]".bright_cyan(),
                modules.len()
            );

            let pattern_bytes = pattern.as_ref().map(|s| parse_hex_pattern(s)).transpose()?;

            let opts = ScanOptions {
                pattern: pattern_bytes.as_deref(),
                verbose: cli.verbose,
                all_modules,
                use_memmap: !no_memmap, // Enable memory mapping by default
            };

            scan_process(&proc, &sys, &opts, &modules)?;
        }
    }
    Ok(())
}
