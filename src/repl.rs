//! REPL (Read-Eval-Print Loop) for interactive memory scanning

use anyhow::Result;
use libmemscan::{
    interactive::{FilterOp, InteractiveScanner},
    process::{MemoryRegionIterator, ProcessHandle, SystemInfo},
    values::{MathOp, Value, ValueType},
};
use owo_colors::OwoColorize;
use std::io::{self, Write};

pub struct Repl<'a> {
    scanner: InteractiveScanner<'a>,
    value_type: ValueType,
}

impl<'a> Repl<'a> {
    pub fn new(
        process: &'a ProcessHandle,
        sys: &SystemInfo,
        value_type: ValueType,
        all_modules: bool,
        modules: &[libmemscan::process::MemoryRegion],
    ) -> Result<Self> {
        // Collect all scannable regions
        let mut regions = Vec::new();
        for region in MemoryRegionIterator::new(process, sys) {
            // Skip if not all_modules and this is a module region
            if !all_modules {
                if modules.iter().any(|m| m.is_superset_of(&region)) {
                    continue;
                }
            }
            regions.push(region);
        }

        let scanner = InteractiveScanner::new(process, regions, value_type);
        Ok(Self {
            scanner,
            value_type,
        })
    }

    pub fn run(&mut self) -> Result<()> {
        println!("{}", "=== Interactive Memory Scanner ===".bright_yellow().bold());
        println!("{} Type 'help' for available commands", "[info]".bright_cyan());
        println!();

        // Perform initial scan
        println!("{} Performing initial scan for {} values...", "[info]".bright_cyan(), format!("{:?}", self.value_type).green());
        let count = self.scanner.initial_scan()?;
        println!(
            "{} Found {} possible addresses across {} regions",
            "[done]".bright_cyan(),
            count.to_string().bright_green(),
            self.scanner.region_count().to_string().bright_green()
        );
        println!();

        loop {
            print!("{} ", ">".bright_yellow().bold());
            io::stdout().flush()?;

            let mut input = String::new();
            io::stdin().read_line(&mut input)?;
            let input = input.trim();

            if input.is_empty() {
                continue;
            }

            match self.handle_command(input) {
                Ok(should_continue) => {
                    if !should_continue {
                        break;
                    }
                }
                Err(e) => {
                    println!("{} {}", "[error]".bright_red(), e);
                }
            }
        }

        Ok(())
    }

    fn handle_command(&mut self, input: &str) -> Result<bool> {
        let parts: Vec<&str> = input.split_whitespace().collect();
        if parts.is_empty() {
            return Ok(true);
        }

        match parts[0] {
            "help" | "h" => {
                self.print_help();
            }
            "list" | "l" => {
                self.list_matches()?;
            }
            "filter" | "f" => {
                if parts.len() < 2 {
                    println!("{} Usage: filter <op> [value]", "[error]".bright_red());
                    println!("  Ops: eq, lt, gt, inc, dec, changed, unchanged");
                } else {
                    self.filter_matches(&parts[1..])?;
                }
            }
            "set" | "s" => {
                if parts.len() < 2 {
                    println!("{} Usage: set <value> [address]", "[error]".bright_red());
                } else {
                    self.set_value(&parts[1..])?;
                }
            }
            "add" | "sub" | "mul" | "div" => {
                if parts.len() < 2 {
                    println!("{} Usage: {} <value> [address]", "[error]".bright_red(), parts[0]);
                } else {
                    self.modify_value(parts[0], &parts[1..])?;
                }
            }
            "quit" | "q" | "exit" => {
                println!("{} Exiting...", "[info]".bright_cyan());
                return Ok(false);
            }
            _ => {
                println!("{} Unknown command: {}", "[error]".bright_red(), parts[0]);
                println!("Type 'help' for available commands");
            }
        }

        Ok(true)
    }

    fn print_help(&self) {
        println!("{}", "Available commands:".bright_yellow().bold());
        println!("  {} - Show this help", "help, h".green());
        println!("  {} - List current matched addresses (max 20)", "list, l".green());
        println!("  {} - Filter addresses", "filter <op> [value]".green());
        println!("    Ops: {} (equals), {} (less than), {} (greater than)", "eq".cyan(), "lt".cyan(), "gt".cyan());
        println!("    Ops: {} (increased), {} (decreased), {} (changed), {} (unchanged)", "inc".cyan(), "dec".cyan(), "changed".cyan(), "unchanged".cyan());
        println!("  {} - Set value at address(es)", "set <value> [address]".green());
        println!("  {} - Add/sub/mul/div value", "add/sub/mul/div <value> [address]".green());
        println!("  {} - Exit the REPL", "quit, q, exit".green());
        println!();
        println!("{} If no address is specified, operation applies to all matches", "[note]".bright_black());
    }

    fn list_matches(&self) -> Result<()> {
        let matches = self.scanner.matches();
        println!("{} matches found", matches.len().to_string().bright_green());

        let display_count = matches.len().min(20);
        for (i, m) in matches.iter().take(display_count).enumerate() {
            let value_str = format_value(&m.current_value);
            let prev_str = m.previous_value.as_ref()
                .map(|v| format!(" (was: {})", format_value(v)))
                .unwrap_or_default();
            println!(
                "  {}: {} = {}{}",
                i.to_string().bright_black(),
                format!("{:016x}", m.address).bright_yellow(),
                value_str.bright_green(),
                prev_str.bright_black()
            );
        }

        if matches.len() > display_count {
            println!("  {} ... and {} more", "[...]".bright_black(), (matches.len() - display_count).to_string().bright_black());
        }

        Ok(())
    }

    fn filter_matches(&mut self, args: &[&str]) -> Result<()> {
        if args.is_empty() {
            anyhow::bail!("Filter operation required");
        }

        let (op, compare_value) = match args[0] {
            "eq" => {
                if args.len() < 2 {
                    anyhow::bail!("Value required for 'eq' filter");
                }
                (FilterOp::Equals, Some(parse_value(args[1], self.value_type)?))
            }
            "lt" => {
                if args.len() < 2 {
                    anyhow::bail!("Value required for 'lt' filter");
                }
                (FilterOp::LessThan, Some(parse_value(args[1], self.value_type)?))
            }
            "gt" => {
                if args.len() < 2 {
                    anyhow::bail!("Value required for 'gt' filter");
                }
                (FilterOp::GreaterThan, Some(parse_value(args[1], self.value_type)?))
            }
            "inc" | "increased" => (FilterOp::Increased, None),
            "dec" | "decreased" => (FilterOp::Decreased, None),
            "changed" => (FilterOp::Changed, None),
            "unchanged" => (FilterOp::Unchanged, None),
            _ => anyhow::bail!("Unknown filter operation: {}", args[0]),
        };

        let before = self.scanner.matches().len();
        let after = self.scanner.filter(op, compare_value)?;
        
        println!(
            "{} Filtered from {} to {} addresses ({} regions)",
            "[done]".bright_cyan(),
            before.to_string().bright_yellow(),
            after.to_string().bright_green(),
            self.scanner.region_count().to_string().bright_green()
        );

        Ok(())
    }

    fn set_value(&mut self, args: &[&str]) -> Result<()> {
        if args.is_empty() {
            anyhow::bail!("Value required");
        }

        let value = parse_value(args[0], self.value_type)?;

        if args.len() > 1 {
            // Set specific address
            let addr = parse_address(args[1])?;
            self.scanner.write_value(addr, value)?;
            println!("{} Set value at {:016x}", "[done]".bright_cyan(), addr);
        } else {
            // Set all addresses
            let count = self.scanner.write_all(value)?;
            println!("{} Set value at {} addresses", "[done]".bright_cyan(), count.to_string().bright_green());
        }

        Ok(())
    }

    fn modify_value(&mut self, op_str: &str, args: &[&str]) -> Result<()> {
        if args.is_empty() {
            anyhow::bail!("Value required");
        }

        let value = parse_value(args[0], self.value_type)?;
        let op = match op_str {
            "add" => MathOp::Add,
            "sub" => MathOp::Subtract,
            "mul" => MathOp::Multiply,
            "div" => MathOp::Divide,
            _ => anyhow::bail!("Unknown operation: {}", op_str),
        };

        if args.len() > 1 {
            // Modify specific address
            let addr = parse_address(args[1])?;
            self.scanner.modify_value(addr, op, value)?;
            println!("{} Modified value at {:016x}", "[done]".bright_cyan(), addr);
        } else {
            // Modify all addresses
            let count = self.scanner.modify_all(op, value)?;
            println!("{} Modified {} addresses", "[done]".bright_cyan(), count.to_string().bright_green());
        }

        Ok(())
    }
}

fn parse_value(s: &str, value_type: ValueType) -> Result<Value> {
    Ok(match value_type {
        ValueType::I8 => Value::I8(s.parse()?),
        ValueType::I16 => Value::I16(s.parse()?),
        ValueType::I32 => Value::I32(s.parse()?),
        ValueType::I64 => Value::I64(s.parse()?),
        ValueType::U8 => Value::U8(s.parse()?),
        ValueType::U16 => Value::U16(s.parse()?),
        ValueType::U32 => Value::U32(s.parse()?),
        ValueType::U64 => Value::U64(s.parse()?),
        ValueType::F32 => Value::F32(s.parse()?),
        ValueType::F64 => Value::F64(s.parse()?),
    })
}

fn parse_address(s: &str) -> Result<usize> {
    // Support hex addresses with 0x prefix
    if let Some(hex) = s.strip_prefix("0x") {
        Ok(usize::from_str_radix(hex, 16)?)
    } else {
        Ok(s.parse()?)
    }
}

fn format_value(value: &Value) -> String {
    match value {
        Value::I8(v) => format!("{}", v),
        Value::I16(v) => format!("{}", v),
        Value::I32(v) => format!("{}", v),
        Value::I64(v) => format!("{}", v),
        Value::U8(v) => format!("{}", v),
        Value::U16(v) => format!("{}", v),
        Value::U32(v) => format!("{}", v),
        Value::U64(v) => format!("{}", v),
        Value::F32(v) => format!("{}", v),
        Value::F64(v) => format!("{}", v),
    }
}
