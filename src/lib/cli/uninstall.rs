use super::{util, ErrorKind, Result};
use crate::{
    diff,
    package::{self, retriever::Retriever},
    project::{self, Application, Package, Project},
    semver,
    solver::Resolver,
};
use clap::ArgMatches;
use colored::Colorize;
use failure::ResultExt;
use slog::Logger;
use std::collections::{BTreeMap, HashSet};

pub fn run(matches: &ArgMatches, logger: &Logger) -> Result<()> {
    util::with_elm_json(&matches, &logger, uninstall_application, uninstall_package)
}

fn uninstall_application(matches: &ArgMatches, logger: &Logger, info: Application) -> Result<()> {
    let strictness = semver::Strictness::Exact;
    let elm_version = info.elm_version();

    let mut retriever: Retriever =
        Retriever::new(&logger, &elm_version.into()).context(ErrorKind::Unknown)?;

    let extras: HashSet<_> = matches
        .values_of_lossy("extra")
        .unwrap_or_else(Vec::new)
        .iter()
        .map(|p| p.parse::<package::Name>().expect("Invalid package name"))
        .collect();

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
        .context(ErrorKind::NoResolution)?;

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

    diff::show(
        diff::Kind::Direct,
        &info.dependencies.direct,
        &deps.0.direct,
    );
    diff::show(
        diff::Kind::Indirect,
        &info.dependencies.indirect,
        &deps.0.indirect,
    );
    diff::show(
        diff::Kind::DirectTest,
        &info.test_dependencies.direct,
        &deps.1.direct,
    );
    diff::show(
        diff::Kind::IndirectTest,
        &info.test_dependencies.indirect,
        &deps.1.indirect,
    );

    let updated = Project::Application(info.with(deps.0, deps.1));
    if util::confirm("Should I make these changes?", &matches)? {
        util::write_elm_json(&updated, &matches)?;
        println!("Saved updated elm.json!");
    } else {
        println!("Aborting!");
    }

    Ok(())
}

fn uninstall_package(matches: &ArgMatches, _logger: &Logger, info: Package) -> Result<()> {
    let extras: HashSet<_> = matches
        .values_of_lossy("extra")
        .unwrap_or_else(Vec::new)
        .iter()
        .map(|p| p.parse::<package::Name>().expect("Invalid package name"))
        .collect();

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

    diff::show(diff::Kind::Regular, &info.dependencies, &new_deps);
    diff::show(diff::Kind::Test, &info.test_dependencies, &new_test_deps);

    let updated = Project::Package(info.with_deps(new_deps, new_test_deps));
    if util::confirm("Should I make these changes?", &matches)? {
        util::write_elm_json(&updated, &matches)?;
        println!("Saved!");
    } else {
        println!("Aborting!");
    }

    Ok(())
}
