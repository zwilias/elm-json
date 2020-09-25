#![warn(unused_extern_crates)]
use std::alloc::System;

#[global_allocator]
static A: System = System;

use cli::ErrorKind;
use colored::Colorize;
use elm_json::cli;
use failure::{Fail, ResultExt};
use slog::{o, Drain, Logger};

fn main() {
    if let Err(e) = run() {
        eprintln!(
            "\n{}\n",
            cli::util::format_header(&e.to_string().to_uppercase()).red()
        );
        e.cause()
            .map(|e| eprintln!("{}", textwrap::fill(&e.to_string(), 80)))
            .unwrap_or(());
        std::process::exit(1);
    }
}

fn run() -> cli::Result<()> {
    ctrlc::set_handler(move || {
        let term = console::Term::stdout();
        let _ = term.show_cursor();
    })
    .context(ErrorKind::Unknown)?;

    let matches = cli::build().get_matches();

    let min_log_level = match matches.occurrences_of("verbose") {
        0 => slog::Level::Warning,
        1 => slog::Level::Info,
        2 => slog::Level::Debug,
        _ => slog::Level::Trace,
    };

    let offline = matches.is_present("offline");

    let logger = make_logger(min_log_level);

    match matches.subcommand() {
        ("solve", Some(matches)) => cli::solve::run(matches, offline, &logger),
        ("upgrade", Some(matches)) => cli::upgrade::run(matches, offline, &logger),
        ("install", Some(matches)) => cli::install::run(matches, offline, &logger),
        ("uninstall", Some(matches)) => cli::uninstall::run(matches, offline, &logger),
        ("new", Some(matches)) => cli::new::run(matches, &logger),
        ("completions", Some(matches)) => cli::completions::run(matches),
        ("tree", Some(matches)) => cli::tree::run(matches, offline, &logger),
        (cmd, matches) => panic!(
            "Received command {} with matches {:#?} but I don't know how to handle this",
            cmd, matches
        ),
    }
}

fn make_logger(min_log_level: slog::Level) -> Logger {
    let decorator = slog_term::TermDecorator::new().build();
    let drain = slog_term::CompactFormat::new(decorator).build().fuse();
    let drain = slog::LevelFilter::new(drain, min_log_level).fuse();
    let drain = slog_async::Async::new(drain).build().fuse();
    slog::Logger::root(drain, o!())
}
