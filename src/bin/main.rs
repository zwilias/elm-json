#![warn(unused_extern_crates)]

use colored::Colorize;
use elm_json::cli;
use failure::{bail, Error};
use slog::{o, Drain, Logger};

fn main() {
    if let Err(e) = run() {
        eprintln!(
            "\n{}\n",
            cli::util::format_header("UNRECOVERABLE ERROR OCCURRED").red()
        );

        eprintln!("{}", e);
        std::process::exit(1);
    }
}

fn run() -> Result<(), Error> {
    let matches = cli::build().get_matches();

    let min_log_level = match matches.occurrences_of("verbose") {
        0 => slog::Level::Warning,
        1 => slog::Level::Info,
        2 => slog::Level::Debug,
        _ => slog::Level::Trace,
    };

    let logger = make_logger(min_log_level);

    match matches.subcommand() {
        ("solve", Some(matches)) => cli::solve::run(matches, &logger),
        ("upgrade", Some(matches)) => cli::upgrade::run(matches, &logger),
        ("install", Some(matches)) => cli::install::run(matches, &logger),
        ("uninstall", Some(matches)) => cli::uninstall::run(matches, &logger),
        ("new", Some(matches)) => cli::new::run(matches, &logger),
        ("completions", Some(matches)) => cli::completions::run(matches),
        _ => bail!("Unsupported command?!"),
    }
}

fn make_logger(min_log_level: slog::Level) -> Logger {
    let decorator = slog_term::TermDecorator::new().build();
    let drain = slog_term::CompactFormat::new(decorator).build().fuse();
    let drain = slog::LevelFilter::new(drain, min_log_level).fuse();
    let drain = slog_async::Async::new(drain).build().fuse();
    slog::Logger::root(drain, o!())
}
