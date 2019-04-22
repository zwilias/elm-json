use super::util;
use crate::{
    package::{self, retriever::Retriever},
    project::{self, Application, Project},
    semver,
    solver::Resolver,
};
use clap::ArgMatches;
use colored::Colorize;
use dialoguer::Confirmation;
use failure::Error;
use serde::ser::Serialize;
use slog::Logger;
use std::{
    collections::HashSet,
    fs::File,
    io::{BufReader, BufWriter},
};

pub fn run(matches: &ArgMatches, logger: &Logger) -> Result<(), Error> {
    let path = matches.value_of("INPUT").unwrap();
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    let info: Project = serde_json::from_reader(reader)?;

    match info {
        Project::Application(app) => uninstall_application(&matches, &logger, &app),
        Project::Package(_pkg) => {
            util::unsupported("Uninstalling dependencies for packages is not yet supported")
        }
    }
}

fn uninstall_application(
    matches: &ArgMatches,
    logger: &Logger,
    info: &Application,
) -> Result<(), Error> {
    let strictness = semver::Strictness::Exact;
    let elm_version = info.elm_version();

    let mut retriever: Retriever = Retriever::new(&logger, elm_version.into())?;

    let extras: Result<HashSet<_>, Error> = matches
        .values_of_lossy("extra")
        .unwrap_or_else(Vec::new)
        .iter()
        .map(|p| p.parse::<package::Name>())
        .collect();
    let extras = extras?;

    retriever.add_preferred_versions(
        info.dependencies
            .indirect
            .iter()
            .filter(|&(k, _)| !extras.contains(&k.clone()))
            .map(|(k, v)| (k.clone().into(), *v))
            .collect(),
    );

    retriever.add_preferred_versions(
        info.test_dependencies
            .indirect
            .iter()
            .filter(|&(k, _)| !extras.contains(&k.clone()))
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
            util::error_out("NO VALID PACKAGE VERSIONS FOUND", e);
            unreachable!()
        });

    let orig_direct = info
        .dependencies
        .direct
        .keys()
        .filter(|&x| !extras.contains(&x.clone()))
        .cloned()
        .collect::<Vec<_>>();

    let deps = project::reconstruct(&orig_direct, res);

    println!(
        "\n{}\n",
        util::format_header("PACKAGE CHANGES READY").green()
    );

    util::show_diff("direct", &info.dependencies.direct, &deps.0.direct);
    util::show_diff("indirect", &info.dependencies.indirect, &deps.0.indirect);
    util::show_diff(
        "direct test",
        &info.test_dependencies.direct,
        &deps.1.direct,
    );
    util::show_diff(
        "indirect test",
        &info.test_dependencies.indirect,
        &deps.1.indirect,
    );

    if matches.is_present("yes")
        || Confirmation::new()
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

        println!("Saved updated elm.json!");
    } else {
        println!("Aborting!");
    }

    Ok(())
}
