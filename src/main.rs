#[cfg(not(target_os = "windows"))]
compile_error!("This program only supports Windows.");

mod cli;
mod memoryapi;
mod process;
mod scanner;

use clap::Parser;
use cli::{Cli, Command};
use owo_colors::OwoColorize;
use process::{open_process, query_system_info};
use scanner::{ScanOptions, scan_process};

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Command::Scan { target, pattern } => {
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
                let pid = process::find_process_by_name(&target)?
                    .ok_or_else(|| anyhow::anyhow!("process with name '{}' not found", target))?;
                println!("{} found pid={}", "[info]".bright_cyan(), pid);
                pid
            };
            let proc = open_process(pid)?;

            let sys = query_system_info();

            let pattern_bytes = pattern.as_ref().map(|s| parse_hex_pattern(s)).transpose()?;

            let opts = ScanOptions {
                pattern: pattern_bytes.as_deref(),
                verbose: cli.verbose,
            };

            scan_process(&proc, &sys, &opts)?;
        }
    }
    Ok(())
}

/// Parse a hex string like "DEADBEEF" or "DEADBEEF" into bytes.
#[cfg(windows)]
fn parse_hex_pattern(s: &str) -> anyhow::Result<Vec<u8>> {
    let filtered: String = s.chars().filter(|c| !c.is_whitespace()).collect();

    if filtered.len() % 2 != 0 {
        anyhow::bail!("hex pattern length must be even");
    }

    let mut bytes = Vec::with_capacity(filtered.len() / 2);
    for i in (0..filtered.len()).step_by(2) {
        let byte_str = &filtered[i..i + 2];
        let b = u8::from_str_radix(byte_str, 16)
            .map_err(|_| anyhow::anyhow!("invalid hex byte '{}'", byte_str))?;
        bytes.push(b);
    }
    Ok(bytes)
}
