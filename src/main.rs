mod cli;
mod server;
mod tools;

use cli::{init_logging, parse_cli, print_usage, CliAction};
use std::env;
use std::error::Error;
use std::io;
use std::process;

fn main() {
    if let Err(err) = try_main() {
        eprintln!("{err}");
        process::exit(1);
    }
}

fn try_main() -> Result<(), Box<dyn Error>> {
    let action = parse_cli(env::args().skip(1))?;

    match action {
        CliAction::Help => {
            print_usage();
            Ok(())
        }
        CliAction::Version => {
            println!("{}", cli::version_string());
            Ok(())
        }
        CliAction::Run { log_level, workdir } => {
            if let Some(workdir) = workdir {
                env::set_current_dir(&workdir).map_err(|err| {
                    io::Error::new(
                        io::ErrorKind::InvalidInput,
                        format!(
                            "Failed to set working directory to {}: {err}",
                            workdir.display()
                        ),
                    )
                })?;
            }
            init_logging(log_level)?;
            server::run_server()?;
            Ok(())
        }
    }
}
