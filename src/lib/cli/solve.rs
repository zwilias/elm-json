use super::util;
use crate::{
    package::{retriever::Retriever, Package},
    project::{AppDependencies, Application, Project},
    semver,
    solver::Resolver,
};
use clap::ArgMatches;
use failure::{format_err, Error};
use slog::Logger;

pub fn run(matches: &ArgMatches, logger: &Logger) -> Result<(), Error> {
    match util::read_elm_json(&matches)? {
        Project::Application(app) => solve_application(&matches, &logger, &app),
        Project::Package(pkg) => solve_package(&matches, &logger, &pkg),
    }
}

fn solve_application(
    matches: &ArgMatches,
    logger: &Logger,
    info: &Application,
) -> Result<(), Error> {
    let deps = &info.dependencies(&semver::Strictness::Exact);
    let indirect = &info.indirect_dependencies();
    let elm_version = info.elm_version();

    let mut retriever: Retriever = Retriever::new(&logger, elm_version.into())?;
    let extras = util::add_extra_deps(&matches, &mut retriever)?;

    retriever.add_preferred_versions(
        indirect
            .iter()
            .filter(|&(k, _)| !extras.contains(&k.clone()))
            .map(|(k, v)| (k.clone().into(), *v))
            .collect(),
    );

    let mut deps: Vec<_> = deps
        .iter()
        .filter(|(k, _)| !extras.contains(k))
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();

    if matches.is_present("test") {
        let test_deps = &info.test_dependencies(&semver::Strictness::Exact);
        let test_deps: Vec<_> = test_deps
            .iter()
            .filter(|(k, _)| !extras.contains(k))
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();

        deps.extend(test_deps);

        retriever.add_preferred_versions(
            info.indirect_test_dependencies()
                .iter()
                .filter(|&(k, _)| !extras.contains(&k.clone()))
                .map(|(k, v)| (k.clone().into(), *v))
                .collect(),
        )
    }

    retriever.add_deps(&deps);

    let res = Resolver::new(&logger, &mut retriever)
        .solve()
        .and_then(|x| {
            serde_json::to_string(&AppDependencies::from(x)).map_err(|e| format_err!("{}", e))
        });
    match res {
        Ok(v) => println!("{}", v),
        Err(e) => util::error_out("NO VALID PACKAGE VERSIONS FOUND", e),
    }
    Ok(())
}

fn solve_package(matches: &ArgMatches, logger: &Logger, info: &Package) -> Result<(), Error> {
    let deps = if matches.is_present("test") {
        info.all_dependencies()?
    } else {
        info.dependencies()
    };

    let mut retriever: Retriever = Retriever::new(&logger, info.elm_version().to_constraint())?;

    if matches.is_present("minimize") {
        retriever.minimize();
    }

    let extras = util::add_extra_deps(&matches, &mut retriever)?;

    let deps: Vec<_> = deps
        .iter()
        .filter(|(k, _)| !extras.contains(k))
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();

    retriever.add_deps(&deps);

    let res = Resolver::new(&logger, &mut retriever)
        .solve()
        .and_then(|x| {
            serde_json::to_string(&AppDependencies::from(x)).map_err(|e| format_err!("{}", e))
        });
    match res {
        Ok(v) => println!("{}", v),
        Err(e) => util::error_out("NO VALID PACKAGE VERSIONS FOUND", e),
    }
    Ok(())
}
