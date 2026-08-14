//! Man page and shell completion generation. The package build calls
//! this through hidden subcommands, so the pages always match the real
//! CLI definitions and cannot drift from `--help`.

use std::path::Path;

/// Write one section-1 man page for the command and one for each
/// visible subcommand, recursively. The page name joins the command
/// path with hyphens: `tisket.1`, `tisket-issue.1`,
/// `tisket-issue-create.1`.
pub fn write_man_pages(cmd: &clap::Command, dir: &Path) -> std::io::Result<()> {
    let name = cmd.get_name().to_string();
    write_pages_rec(cmd, &name, dir)
}

fn write_pages_rec(cmd: &clap::Command, path_name: &str, dir: &Path) -> std::io::Result<()> {
    // clap's builder wants a 'static name without the `string` feature.
    // The generator runs once per page and exits, so the leak is bounded.
    let leaked: &'static str = Box::leak(path_name.to_string().into_boxed_str());
    let man = clap_mangen::Man::new(cmd.clone().name(leaked));
    let mut buf: Vec<u8> = Vec::new();
    man.render(&mut buf)?;
    std::fs::write(dir.join(format!("{path_name}.1")), buf)?;
    for sub in cmd.get_subcommands() {
        if sub.is_hide_set() || sub.get_name() == "help" {
            continue;
        }
        let child = format!("{path_name}-{}", sub.get_name());
        write_pages_rec(sub, &child, dir)?;
    }
    Ok(())
}
