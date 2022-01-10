#![warn(unused_extern_crates)]
use std::alloc::System;

#[global_allocator]
static A: System = System;

use anyhow::{Context, Result};
use cli::Kind;
use colored::Colorize;
use elm_json::cli;
use tracing::Level;
use tracing_subscriber::{self, filter::LevelFilter, layer::SubscriberExt};

fn main() {
    if let Err(e) = run() {
        eprintln!(
            "\n{}\n",
            cli::util::format_header(&e.to_string().to_uppercase()).red()
        );
        e.source()
            .map(|e| eprintln!("{}", textwrap::fill(&e.to_string(), 80)))
            .unwrap_or(());
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    ctrlc::set_handler(move || {
        let term = console::Term::stdout();
        let _ = term.show_cursor();
    })
    .context(Kind::Unknown)?;

    let matches = cli::build().get_matches();

    let min_level = match matches.occurrences_of("verbose") {
        0 => Level::WARN,
        1 => Level::INFO,
        2 => Level::DEBUG,
        _ => Level::TRACE,
    };

    let subscriber = tracing_subscriber::registry::Registry::default()
        .with(LevelFilter::from_level(min_level))
        .with(tracing_subscriber::fmt::Layer::default());
    tracing::subscriber::set_global_default(subscriber).expect("Failed to set global subscriber");

    let offline = matches.is_present("offline");

    match matches.subcommand() {
        ("solve", Some(matches)) => cli::solve::run(matches, offline),
        ("upgrade", Some(matches)) => cli::upgrade::run(matches, offline),
        ("install", Some(matches)) => cli::install::run(matches, offline),
        ("uninstall", Some(matches)) => cli::uninstall::run(matches, offline),
        ("new", Some(matches)) => cli::new::run(matches),
        ("completions", Some(matches)) => cli::completions::run(matches),
        ("tree", Some(matches)) => cli::tree::run(matches, offline),
        (cmd, matches) => panic!(
            "Received command {} with matches {:#?} but I don't know how to handle this",
            cmd, matches
        ),
    }
}
