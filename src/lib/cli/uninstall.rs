use super::util;
use crate::{
    package::{self, retriever::Retriever},
    project::{self, Application, Package, Project},
    semver,
    solver::Resolver,
};
use clap::ArgMatches;
use colored::Colorize;
use dialoguer::Confirmation;
use failure::Error;
use slog::Logger;
use std::collections::{BTreeMap, HashSet};

pub fn run(matches: &ArgMatches, logger: &Logger) -> Result<(), Error> {
    util::with_elm_json(&matches, &logger, uninstall_application, uninstall_package)
}

fn uninstall_application(
    matches: &ArgMatches,
    logger: &Logger,
    info: &Application,
) -> Result<(), Error> {
    let strictness = semver::Strictness::Exact;
    let elm_version = info.elm_version();

    let mut retriever: Retriever = Retriever::new(&logger, &elm_version.into())?;

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
            .map(|(k, v)| (k.clone().into(), *v)),
    );

    retriever.add_preferred_versions(
        info.test_dependencies
            .indirect
            .iter()
            .filter(|&(k, _)| !extras.contains(&k.clone()))
            .map(|(k, v)| (k.clone().into(), *v)),
    );

    retriever.add_deps(
        info.dependencies(&strictness)
            .iter()
            .filter(|(k, _)| !extras.contains(k)),
    );

    retriever.add_deps(
        info.test_dependencies(&strictness)
            .iter()
            .filter(|(k, _)| !extras.contains(k)),
    );

    let res = Resolver::new(&logger, &mut retriever)
        .solve()
        .unwrap_or_else(|e| {
            util::error_out("NO VALID PACKAGE VERSIONS FOUND", &e);
            unreachable!()
        });

    let orig_direct = info
        .dependencies
        .direct
        .keys()
        .filter(|&x| !extras.contains(&x.clone()))
        .cloned()
        .collect::<Vec<_>>();

    let deps = project::reconstruct(&orig_direct, &res);

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

    let updated = Project::Application(info.with_deps(deps.0).with_test_deps(deps.1));
    if matches.is_present("yes")
        || Confirmation::new()
            .with_text("Should I make these changes?")
            .interact()?
    {
        util::write_elm_json(&updated, &matches)?;
        println!("Saved updated elm.json!");
    } else {
        println!("Aborting!");
    }

    Ok(())
}

fn uninstall_package(matches: &ArgMatches, _logger: &Logger, info: &Package) -> Result<(), Error> {
    let extras: Result<HashSet<_>, Error> = matches
        .values_of_lossy("extra")
        .unwrap_or_else(Vec::new)
        .iter()
        .map(|p| p.parse::<package::Name>())
        .collect();
    let extras = extras?;

    let new_deps: BTreeMap<_, _> = info
        .dependencies
        .iter()
        .filter(|&(k, _)| !extras.contains(&k.clone()))
        .map(|(k, v)| (k.clone(), *v))
        .collect();

    let new_test_deps: BTreeMap<_, _> = info
        .test_dependencies
        .iter()
        .filter(|&(k, _)| !extras.contains(&k.clone()))
        .map(|(k, v)| (k.clone(), *v))
        .collect();

    println!(
        "\n{}\n",
        util::format_header("PACKAGE CHANGES READY").green()
    );

    util::show_diff("", &info.dependencies, &new_deps);
    util::show_diff("test", &info.test_dependencies, &new_test_deps);

    let updated = Project::Package(info.with_deps(new_deps, new_test_deps));
    if matches.is_present("yes")
        || Confirmation::new()
            .with_text("Should I make these changes?")
            .interact()?
    {
        util::write_elm_json(&updated, &matches)?;
        println!("Saved!");
    } else {
        println!("Aborting!");
    }

    Ok(())
}
