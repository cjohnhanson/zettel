use std::process::ExitCode;

use clap::Parser;

fn main() -> ExitCode {
    // Exit cleanly on broken pipe instead of panicking.
    sigpipe::reset();

    let args = zettel::cli::Args::parse();

    match zettel::cli::run(args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::from(1)
        }
    }
}
