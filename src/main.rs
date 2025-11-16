#[cfg(not(target_os = "windows"))]
compile_error!("This program only supports Windows.");

use clap::Parser;
use memscan::{cli::{Cli, Command}, process, scanner::{ScanOptions, scan_process}, parse_hex_pattern};
use owo_colors::OwoColorize;
use process::{open_process, query_system_info, find_process_by_name};

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Command::Scan {
            target,
            pattern,
            all_modules,
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

            let modules = process::get_process_module_regions(&proc)?;
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
            };

            scan_process(&proc, &sys, &opts, &modules)?;
        }
    }
    Ok(())
}
