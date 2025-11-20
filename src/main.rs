use clap::{Parser, Subcommand, ValueHint, builder::styling::AnsiColor};
use libmemscan::{
    parse_hex_pattern,
    process::{find_process_by_name, get_process_module_regions, open_process, query_system_info},
    scanner::{ScanOptions, scan_process},
    values::ValueType,
};
use owo_colors::OwoColorize;

mod repl;

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
    #[command(alias = "s")]
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
    },
    /// Interactive mode for iterative memory scanning and modification
    #[command(alias = "i")]
    Interactive {
        /// Target process executable name or id (e.g. "notepad", "notepad.exe", or 1234)
        target: String,

        /// Value type to scan for (i8, i16, i32, i64, u8, u16, u32, u64, f32, f64)
        #[arg(short = 't', long, default_value = "i32")]
        value_type: String,

        /// Scan all modules, including those not originating from the target process
        /// (by default, only the process's own modules are scanned)
        #[arg(long)]
        all_modules: bool,
    },
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Command::Scan {
            target,
            pattern,
            all_modules,
        } => {
            let pid = resolve_target(&target)?;
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

            let Some(pattern) = pattern.as_ref().map(|s| parse_hex_pattern(s)).transpose()? else {
                anyhow::bail!("a hex pattern must be specified for scanning");
            };

            let opts = ScanOptions {
                verbose: cli.verbose,
                all_modules,
            };

            scan_process(&proc, &sys, &pattern, &opts, &modules)?;
        }
        Command::Interactive {
            target,
            value_type,
            all_modules,
        } => {
            let pid = resolve_target(&target)?;
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

            let vtype = parse_value_type(&value_type)?;
            let mut repl = repl::Repl::new(&proc, &sys, vtype, all_modules, &modules)?;
            repl.run()?;
        }
    }
    Ok(())
}

fn resolve_target(target: &str) -> anyhow::Result<u32> {
    if target.chars().all(|c| c.is_ascii_digit()) {
        let pid: u32 = target.parse()?;
        println!("{} target pid={}", "[info]".bright_cyan(), pid);
        Ok(pid)
    } else {
        println!(
            "{} looking up process by name: {}",
            "[info]".bright_cyan(),
            target
        );
        let pid = find_process_by_name(&target)?
            .ok_or_else(|| anyhow::anyhow!("process with name '{}' not found", target))?;
        println!("{} found pid={}", "[info]".bright_cyan(), pid);
        Ok(pid)
    }
}

fn parse_value_type(s: &str) -> anyhow::Result<ValueType> {
    Ok(match s.to_lowercase().as_str() {
        "i8" => ValueType::I8,
        "i16" => ValueType::I16,
        "i32" => ValueType::I32,
        "i64" => ValueType::I64,
        "u8" => ValueType::U8,
        "u16" => ValueType::U16,
        "u32" => ValueType::U32,
        "u64" => ValueType::U64,
        "f32" => ValueType::F32,
        "f64" => ValueType::F64,
        _ => anyhow::bail!(
            "Unknown value type: {}. Valid types: i8, i16, i32, i64, u8, u16, u32, u64, f32, f64",
            s
        ),
    })
}
