#![warn(unused_extern_crates)]

use elm_json::cli;
use failure::Error;
use slog::{o, Drain};

fn main() -> Result<(), Error> {
    let matches = cli::build().get_matches();

    let min_log_level = match matches.occurrences_of("verbose") {
        0 => slog::Level::Warning,
        1 => slog::Level::Info,
        2 => slog::Level::Debug,
        _ => slog::Level::Trace,
    };

    let decorator = slog_term::TermDecorator::new().build();
    let drain = slog_term::CompactFormat::new(decorator).build().fuse();
    let drain = slog::LevelFilter::new(drain, min_log_level).fuse();
    let drain = slog_async::Async::new(drain).build().fuse();
    let logger = slog::Logger::root(drain, o!());

    if let Some(matches) = matches.subcommand_matches("solve") {
        cli::solve::run(matches, &logger)
    } else if let Some(matches) = matches.subcommand_matches("upgrade") {
        cli::upgrade::run(matches, &logger)
    } else if let Some(matches) = matches.subcommand_matches("install") {
        cli::install::run(matches, &logger)
    } else if let Some(matches) = matches.subcommand_matches("uninstall") {
        cli::uninstall::run(matches, &logger)
    } else if let Some(matches) = matches.subcommand_matches("new") {
        cli::new::run(matches, &logger)
    } else if let Some(matches) = matches.subcommand_matches("completions") {
        cli::completions::run(matches)
    } else {
        unreachable!();
    }
}
