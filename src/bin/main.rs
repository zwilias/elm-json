extern crate clap;
extern crate colored;
extern crate dialoguer;
extern crate elm_json;
extern crate serde_json;
extern crate slog;
extern crate slog_async;
extern crate slog_term;
extern crate textwrap;

use clap::{App, Arg, ArgMatches, SubCommand};
use colored::Colorize;
use dialoguer::Confirmation;
use elm_json::{
    package::{retriever::Retriever, Package},
    project::{self, AppDependencies, Application, Project},
    semver,
    solver::Resolver,
};
use failure::{bail, format_err, Error};
use serde::ser::Serialize;
use slog::{o, Drain, Logger};
use std::{
    collections::{BTreeMap, HashSet},
    fs::File,
    io::{BufReader, BufWriter},
};

fn main() -> Result<(), Error> {
    let matches = App::new("elm-json")
        .version(env!("CARGO_PKG_VERSION"))
        .about("Deal with your elm.json")
        .arg(
            Arg::with_name("verbose")
                .short("v")
                .long("verbose")
                .multiple(true)
                .help("Sets the level of verbosity"),
        )
        .subcommand(
            SubCommand::with_name("upgrade")
                .about("Bring your dependencies up to date")
                .arg(
                    Arg::with_name("unsafe")
                        .help("Allow major versions bumps")
                        .long("unsafe"),
                )
                .arg(
                    Arg::with_name("INPUT")
                        .help("The elm.json file to upgrade")
                        .default_value("elm.json"),
                ),
        )
        .subcommand(
            SubCommand::with_name("install")
                .about("Install a package")
                .arg(
                    Arg::with_name("test")
                        .help("Install as a test-dependency")
                        .long("test"),
                )
                .arg(
                    Arg::with_name("extra")
                        .help("Package to install, e.g. elm/core or elm/core@1.0.2")
                        .takes_value(true)
                        .value_name("PACKAGE")
                        .required(true)
                        .multiple(true),
                )
                .arg(
                    Arg::with_name("INPUT")
                        .help("The elm.json file to upgrade")
                        .last(true)
                        .default_value("elm.json"),
                ),
        )
        .subcommand(
            SubCommand::with_name("uninstall")
                .about("Uninstall a package")
                .arg(
                    Arg::with_name("extra")
                        .help("Package to uninstall, e.g. elm/html")
                        .takes_value(true)
                        .value_name("PACKAGE")
                        .required(true)
                        .multiple(true),
                )
                .arg(
                    Arg::with_name("INPUT")
                        .help("The elm.json file to upgrade")
                        .last(true)
                        .default_value("elm.json"),
                ),
        )
        .subcommand(
            SubCommand::with_name("solve")
                .about("Figure out a solution given the version constraints in your elm.json")
                .arg(
                    Arg::with_name("test")
                        .help("Promote test-dependencies to top-level dependencies")
                        .long("test"),
                )
                .arg(
                    Arg::with_name("extra")
                        .short("e")
                        .long("extra")
                        .help("Specify extra dependencies, e.g. elm/core or elm/core@1.0.2")
                        .takes_value(true)
                        .value_name("PACKAGE")
                        .multiple(true),
                )
                .arg(
                    Arg::with_name("INPUT")
                        .help("The elm.json file to solve")
                        .default_value("elm.json"),
                ),
        )
        .get_matches();

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
        solve(matches, &logger)
    } else if let Some(matches) = matches.subcommand_matches("upgrade") {
        upgrade(matches, &logger)
    } else if let Some(matches) = matches.subcommand_matches("install") {
        install(matches, &logger)
    } else if let Some(matches) = matches.subcommand_matches("uninstall") {
        uninstall(matches, &logger)
    } else {
        println!("I need a command!\n\nTry running with the --help flag for more information.");
        Ok(())
    }
}

fn solve(matches: &ArgMatches, logger: &Logger) -> Result<(), Error> {
    let path = matches.value_of("INPUT").unwrap();
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    let info: Project = serde_json::from_reader(reader)?;

    match info {
        Project::Application(app) => solve_application(&matches, &logger, &app),
        Project::Package(pkg) => solve_package(&matches, &logger, &pkg),
    }
}

fn upgrade(matches: &ArgMatches, logger: &Logger) -> Result<(), Error> {
    let path = matches.value_of("INPUT").unwrap();
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    let info: Project = serde_json::from_reader(reader)?;

    match info {
        Project::Application(app) => upgrade_application(&matches, &logger, &app),
        Project::Package(_pkg) => bail!("TODO: Implement upgrade for package"),
    }
}

fn install(matches: &ArgMatches, logger: &Logger) -> Result<(), Error> {
    let path = matches.value_of("INPUT").unwrap();
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    let info: Project = serde_json::from_reader(reader)?;

    match info {
        Project::Application(app) => install_application(&matches, &logger, &app),
        Project::Package(_pkg) => bail!("TODO: Implement install for package"),
    }
}

fn uninstall(matches: &ArgMatches, logger: &Logger) -> Result<(), Error> {
    let path = matches.value_of("INPUT").unwrap();
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    let info: Project = serde_json::from_reader(reader)?;

    match info {
        Project::Application(app) => uninstall_application(&matches, &logger, &app),
        Project::Package(_pkg) => bail!("TODO: Implement uninstall for package"),
    }
}

fn uninstall_application(
    matches: &ArgMatches,
    logger: &Logger,
    info: &Application,
) -> Result<(), Error> {
    let strictness = semver::Strictness::Exact;
    let elm_version = info.elm_version();

    let mut retriever: Retriever = Retriever::new(&logger, elm_version.into());
    retriever.fetch_versions()?;

    let extras: HashSet<String> = matches
        .values_of_lossy("extra")
        .unwrap_or_else(Vec::new)
        .iter()
        .cloned()
        .collect();

    retriever.add_preferred_versions(
        info.dependencies
            .indirect
            .iter()
            .filter(|(k, _)| !extras.contains(k.clone()))
            .map(|(k, v)| (k.clone().into(), *v))
            .collect(),
    );

    retriever.add_preferred_versions(
        info.test_dependencies
            .indirect
            .iter()
            .filter(|(k, _)| !extras.contains(k.clone()))
            .map(|(k, v)| (k.clone().into(), *v))
            .collect(),
    );

    let deps: Vec<_> = info
        .dependencies(&strictness)
        .iter()
        .filter(|(k, _)| !extras.contains(k))
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();

    retriever.add_deps(&deps);

    let deps: Vec<_> = info
        .test_dependencies(&strictness)
        .iter()
        .filter(|(k, _)| !extras.contains(k))
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();

    retriever.add_deps(&deps);

    let res = Resolver::new(&logger, &mut retriever)
        .solve()
        .unwrap_or_else(|e| {
            error_out("NO VALID PACKAGE VERSIONS FOUND", e);
            unreachable!()
        });

    let orig_direct = info
        .dependencies
        .direct
        .keys()
        .filter(|x| !extras.contains(x.clone()))
        .cloned()
        .collect::<Vec<_>>();

    let deps = project::reconstruct(&orig_direct, res);

    println!("\n{}\n", format_header("PACKAGE CHANGES READY").green());

    show_diff("direct", &info.dependencies.direct, &deps.0.direct);
    show_diff("indirect", &info.dependencies.indirect, &deps.0.indirect);
    show_diff(
        "direct test",
        &info.test_dependencies.direct,
        &deps.1.direct,
    );
    show_diff(
        "indirect test",
        &info.test_dependencies.indirect,
        &deps.1.indirect,
    );

    if Confirmation::new()
        .with_text("Should I make these changes?")
        .interact()?
    {
        let path = matches.value_of("INPUT").unwrap();
        let file = File::create(path)?;
        let writer = BufWriter::new(file);
        let formatter = serde_json::ser::PrettyFormatter::with_indent(b"    ");
        let mut serializer = serde_json::Serializer::with_formatter(writer, formatter);
        let val = Project::Application(info.with_deps(deps.0).with_test_deps(deps.1));
        val.serialize(&mut serializer)?;
    } else {
        println!("Aborting!");
    }

    Ok(())
}

fn install_application(
    matches: &ArgMatches,
    logger: &Logger,
    info: &Application,
) -> Result<(), Error> {
    let strictness = semver::Strictness::Exact;
    let elm_version = info.elm_version();

    let mut retriever: Retriever = Retriever::new(&logger, elm_version.into());
    retriever.fetch_versions()?;

    let extras = add_extra_deps(matches, &mut retriever)?;

    retriever.add_preferred_versions(
        info.dependencies
            .indirect
            .iter()
            .filter(|(k, _)| !extras.contains(k.clone()))
            .map(|(k, v)| (k.clone().into(), *v))
            .collect(),
    );

    retriever.add_preferred_versions(
        info.test_dependencies
            .indirect
            .iter()
            .filter(|(k, _)| !extras.contains(k.clone()))
            .map(|(k, v)| (k.clone().into(), *v))
            .collect(),
    );

    let deps: Vec<_> = info
        .dependencies(&strictness)
        .iter()
        .filter(|(k, _)| !extras.contains(k))
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();

    retriever.add_deps(&deps);

    let deps: Vec<_> = info
        .test_dependencies(&strictness)
        .iter()
        .filter(|(k, _)| !extras.contains(k))
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();

    retriever.add_deps(&deps);

    let res = Resolver::new(&logger, &mut retriever)
        .solve()
        .unwrap_or_else(|e| {
            error_out("NO VALID PACKAGE VERSIONS FOUND", e);
            unreachable!()
        });

    let extra_direct: Vec<_> = if matches.is_present("test") {
        Vec::new()
    } else {
        extras.iter().map(|x| x.clone()).collect()
    };

    let mut orig_direct = info
        .dependencies
        .direct
        .keys()
        .filter(|x| !extras.contains(x.clone()))
        .map(|x| x.clone())
        .collect::<Vec<_>>();
    orig_direct.extend(extra_direct);

    let deps = project::reconstruct(&orig_direct, res);

    println!("\n{}\n", format_header("PACKAGE CHANGES READY").green());

    show_diff("direct", &info.dependencies.direct, &deps.0.direct);
    show_diff("indirect", &info.dependencies.indirect, &deps.0.indirect);
    show_diff(
        "direct test",
        &info.test_dependencies.direct,
        &deps.1.direct,
    );
    show_diff(
        "indirect test",
        &info.test_dependencies.indirect,
        &deps.1.indirect,
    );

    if Confirmation::new()
        .with_text("Should I make these changes?")
        .interact()?
    {
        let path = matches.value_of("INPUT").unwrap();
        let file = File::create(path)?;
        let writer = BufWriter::new(file);
        let formatter = serde_json::ser::PrettyFormatter::with_indent(b"    ");
        let mut serializer = serde_json::Serializer::with_formatter(writer, formatter);
        let val = Project::Application(info.with_deps(deps.0).with_test_deps(deps.1));
        val.serialize(&mut serializer)?;
    } else {
        println!("Aborting!");
    }

    Ok(())
}

fn upgrade_application(
    matches: &ArgMatches,
    logger: &Logger,
    info: &Application,
) -> Result<(), Error> {
    let strictness = if matches.is_present("unsafe") {
        semver::Strictness::Unsafe
    } else {
        semver::Strictness::Safe
    };
    let elm_version = info.elm_version();

    let mut retriever: Retriever = Retriever::new(&logger, elm_version.into());
    retriever.fetch_versions()?;

    retriever.add_deps(&info.dependencies(&strictness));
    retriever.add_deps(&info.test_dependencies(&strictness));

    let res = Resolver::new(&logger, &mut retriever)
        .solve()
        .unwrap_or_else(|e| {
            error_out("NO VALID PACKAGE VERSIONS FOUND", e);
            unreachable!()
        });

    let deps = project::reconstruct(
        &info.dependencies.direct.keys().map(|x| x.clone()).collect(),
        res,
    );

    if deps.0 == info.dependencies {
        println!("\n{}\n", format_header("PACKAGES UP TO DATE").green());
        println!("All your dependencies appear to be up to date!");
        return Ok(());
    }

    println!("\n{}\n", format_header("PACKAGE UPGRADES FOUND").green());

    show_diff("direct", &info.dependencies.direct, &deps.0.direct);
    show_diff("indirect", &info.dependencies.indirect, &deps.0.indirect);
    show_diff(
        "direct test",
        &info.test_dependencies.direct,
        &deps.1.direct,
    );
    show_diff(
        "indirect test",
        &info.test_dependencies.indirect,
        &deps.1.indirect,
    );

    if Confirmation::new()
        .with_text("Should I make these changes?")
        .interact()?
    {
        let path = matches.value_of("INPUT").unwrap();
        let file = File::create(path)?;
        let writer = BufWriter::new(file);
        let formatter = serde_json::ser::PrettyFormatter::with_indent(b"    ");
        let mut serializer = serde_json::Serializer::with_formatter(writer, formatter);
        let val = Project::Application(info.with_deps(deps.0).with_test_deps(deps.1));
        val.serialize(&mut serializer)?;
    } else {
        println!("Aborting!");
    }

    Ok(())
}

pub fn show_diff(
    title: &str,
    left: &BTreeMap<String, semver::Version>,
    right: &BTreeMap<String, semver::Version>,
) {
    let it = diff(&left, &right);
    if !it.is_empty() {
        println!(
            "I want to make some changes to your {} dependencies\n",
            title.bold()
        );
        it.print();
        println!("");
    }
}

impl Diff {
    pub fn is_empty(&self) -> bool {
        self.only_left.is_empty() && self.only_right.is_empty() && self.changed.is_empty()
    }

    pub fn print(&self) {
        for (k, v) in self.only_left.iter() {
            println!("- {} {} {}", "[DEL]".yellow(), k, v);
        }

        for (k, v) in self.only_right.iter() {
            println!("- {} {} {}", "[ADD]".green(), k, v);
        }

        for (k, o, n) in self.changed.iter() {
            println!("- {} {} {} -> {}", "[CHG]".blue(), k, o, n);
        }
    }
}

struct Diff {
    only_left: Vec<(String, semver::Version)>,
    only_right: Vec<(String, semver::Version)>,
    changed: Vec<(String, semver::Version, semver::Version)>,
}

fn diff(
    left: &BTreeMap<String, semver::Version>,
    right: &BTreeMap<String, semver::Version>,
) -> Diff {
    let mut only_left = Vec::new();
    let mut only_right = Vec::new();
    let mut changed = Vec::new();

    let mut iter_left = left.iter();
    let mut iter_right = right.iter();

    let mut left = iter_left.next();
    let mut right = iter_right.next();

    while let (Some((left_name, left_version)), Some((right_name, right_version))) = (left, right) {
        if left_name == right_name {
            if left_version != right_version {
                changed.push((left_name.clone(), *left_version, *right_version))
            }

            left = iter_left.next();
            right = iter_right.next();
            continue;
        }

        if left_name < right_name {
            only_left.push((left_name.clone(), *left_version));
            left = iter_left.next();
            continue;
        }

        if left_name > right_name {
            only_right.push((right_name.clone(), *right_version));
            right = iter_right.next();
            continue;
        }
    }

    while let Some((name, version)) = left {
        only_left.push((name.clone(), *version));
        left = iter_left.next();
    }

    while let Some((name, version)) = right {
        only_right.push((name.clone(), *version));
        right = iter_right.next();
    }

    Diff {
        only_left,
        only_right,
        changed,
    }
}

fn add_extra_deps(
    matches: &ArgMatches,
    retriever: &mut Retriever,
) -> Result<HashSet<String>, Error> {
    let mut extras = HashSet::new();

    for dep in matches
        .values_of_lossy("extra")
        .unwrap_or_else(Vec::new)
        .iter()
    {
        let parts: Vec<&str> = dep.split('@').collect();
        match parts.as_slice() {
            [name] => {
                retriever.add_dep(&name.to_string(), &None);
                extras.insert(name.to_string());
            }
            [name, version] => {
                let version: semver::Version = version.parse()?;
                retriever.add_dep(&name.to_string(), &Some(version));
                extras.insert(name.to_string());
            }
            _ => unreachable!(),
        }
    }
    Ok(extras)
}

fn solve_application(
    matches: &ArgMatches,
    logger: &Logger,
    info: &Application,
) -> Result<(), Error> {
    let deps = &info.dependencies(&semver::Strictness::Exact);
    let indirect = &info.indirect_dependencies();
    let elm_version = info.elm_version();

    let mut retriever: Retriever = Retriever::new(&logger, elm_version.into());
    let extras = add_extra_deps(&matches, &mut retriever)?;

    retriever.fetch_versions()?;
    retriever.add_preferred_versions(
        indirect
            .iter()
            .filter(|(k, _)| !extras.contains(k.clone()))
            .map(|(k, v)| (k.clone().into(), *v))
            .collect(),
    );

    let deps: Vec<_> = deps
        .iter()
        .filter(|(k, _)| !extras.contains(k))
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();

    retriever.add_deps(&deps);

    let res = Resolver::new(&logger, &mut retriever)
        .solve()
        .and_then(|x| {
            serde_json::to_string(&AppDependencies::new(x)).map_err(|e| format_err!("{}", e))
        });
    match res {
        Ok(v) => println!("{}", v),
        Err(e) => error_out("NO VALID PACKAGE VERSIONS FOUND", e),
    }
    Ok(())
}

fn error_out(msg: &str, e: Error) {
    println!("\n{}", format_header(msg).cyan());
    println!("\n{}", textwrap::fill(&e.to_string(), 80));
    std::process::exit(1)
}

fn format_header(x: &str) -> String {
    format!("-- {} {}", x, "-".repeat(80 - 4 - x.len()))
}

fn solve_package(matches: &ArgMatches, logger: &Logger, info: &Package) -> Result<(), Error> {
    let deps = if matches.is_present("test") {
        info.all_dependencies()?
    } else {
        info.dependencies()
    };

    let mut retriever: Retriever = Retriever::new(&logger, info.elm_version().to_constraint());
    retriever.fetch_versions()?;
    let extras = add_extra_deps(&matches, &mut retriever)?;

    let deps: Vec<_> = deps
        .iter()
        .filter(|(k, _)| !extras.contains(k))
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();

    retriever.add_deps(&deps);

    let res = Resolver::new(&logger, &mut retriever)
        .solve()
        .and_then(|x| {
            serde_json::to_string(&AppDependencies::new(x)).map_err(|e| format_err!("{}", e))
        });
    match res {
        Ok(v) => println!("{}", v),
        Err(e) => error_out("NO VALID PACKAGE VERSIONS FOUND", e),
    }
    Ok(())
}
