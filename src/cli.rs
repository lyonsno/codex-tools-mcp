use std::env;
use std::error::Error;
use std::path::PathBuf;

use env_logger::Builder as LoggerBuilder;

#[derive(Debug)]
pub enum CliAction {
    Run {
        log_level: Option<String>,
        workdir: Option<PathBuf>,
    },
    Help,
    Version,
}

pub fn parse_cli<I>(args: I) -> Result<CliAction, String>
where
    I: Iterator<Item = String>,
{
    let mut log_level = None;
    let mut workdir = None;
    let mut iter = args.peekable();

    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "-h" | "--help" => return Ok(CliAction::Help),
            "-V" | "--version" => return Ok(CliAction::Version),
            _ => {
                if let Some(level) = arg.strip_prefix("--log-level=") {
                    log_level = Some(level.to_string());
                } else if arg == "--log-level" {
                    let value = iter
                        .next()
                        .ok_or_else(|| "--log-level requires a value".to_string())?;
                    log_level = Some(value);
                } else if let Some(path) = arg.strip_prefix("--workdir=") {
                    if path.is_empty() {
                        return Err("--workdir requires a value".to_string());
                    }
                    workdir = Some(PathBuf::from(path));
                } else if arg == "--workdir" {
                    let value = iter
                        .next()
                        .ok_or_else(|| "--workdir requires a value".to_string())?;
                    workdir = Some(PathBuf::from(value));
                } else {
                    return Err(format!("Unknown argument: {arg}"));
                }
            }
        }
    }

    Ok(CliAction::Run { log_level, workdir })
}

pub fn print_usage() {
    println!("Usage: codex-tools-mcp [OPTIONS]\n\nOptions:\n  --log-level <level>   Override default log level (info)\n  --workdir <path>      Set process working directory before serving\n  -V, --version         Print version information\n  -h, --help            Print this help message");
}

pub fn init_logging(log_level: Option<String>) -> Result<(), Box<dyn Error>> {
    let mut builder = LoggerBuilder::from_env(env_logger::Env::default().default_filter_or("info"));
    if let Some(spec) = log_level {
        builder.parse_filters(&spec);
    }
    builder.format_timestamp(None);
    builder.try_init()?;
    Ok(())
}

pub fn version_string() -> String {
    format!("codex-tools-mcp {}", env!("CARGO_PKG_VERSION"))
}
