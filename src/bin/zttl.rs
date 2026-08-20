//! The short name, so `uvx zttl` and `npx zttl` work.
//!
//! maturin ties an installed command name to the Cargo bin name, and
//! refuses a `[project.scripts]` entry beside a binary. A wheel
//! published as `zttl` therefore needs a `zttl` command, or
//! `uvx zttl` errors and tells the reader to type
//! `uvx --from zttl zettel` forever.
//!
//! This execs `zettel` beside it rather than carrying a second copy.
//! Both names then work from one install, which is what the naming
//! scheme asks for.
use std::os::unix::process::CommandExt;

fn main() -> std::process::ExitCode {
    let Ok(me) = std::env::current_exe() else {
        eprintln!("zttl: cannot resolve its own path");
        return std::process::ExitCode::FAILURE;
    };
    let real = me.with_file_name("zettel");
    let err = std::process::Command::new(&real)
        .args(std::env::args_os().skip(1))
        .exec();
    eprintln!("zttl: cannot run {}: {err}", real.display());
    std::process::ExitCode::FAILURE
}
