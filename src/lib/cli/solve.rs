use super::{util, Kind};
use crate::{
    package::{retriever::Retriever, Package},
    project::{AppDependencies, Application},
    semver,
    solver::Resolver,
};
use anyhow::{Context, Result};
use clap::ArgMatches;

pub fn run(matches: &ArgMatches, offline: bool) -> Result<()> {
    util::with_elm_json(matches, offline, solve_application, solve_package)
}

fn solve_application(matches: &ArgMatches, offline: bool, info: Application) -> Result<()> {
    let deps = &info.dependencies(&semver::Strictness::Exact);
    let elm_version = info.elm_version();

    let mut retriever: Retriever =
        Retriever::new(&elm_version.into(), offline).context(Kind::Unknown)?;
    let extras = util::add_extra_deps(matches, &mut retriever);

    retriever.add_preferred_versions(
        info.dependencies
            .indirect
            .iter()
            .filter(|&(k, _)| !extras.contains(&k.clone()))
            .map(|(k, v)| (k.clone().into(), *v)),
    );

    retriever.add_deps(deps.iter().filter(|(k, _)| !extras.contains(k)));

    if matches.is_present("test") {
        retriever.add_deps(
            info.test_dependencies(&semver::Strictness::Exact)
                .iter()
                .filter(|(k, _)| !extras.contains(k)),
        );

        retriever.add_preferred_versions(
            info.test_dependencies
                .indirect
                .iter()
                .filter(|&(k, _)| !extras.contains(&k.clone()))
                .map(|(k, v)| (k.clone().into(), *v)),
        )
    }

    Resolver::new(&mut retriever)
        .solve()
        .context(Kind::NoResolution)
        .and_then(|x| serde_json::to_string(&AppDependencies::from(x)).context(Kind::Unknown))
        .map(|v| println!("{}", v))?;
    Ok(())
}

fn solve_package(matches: &ArgMatches, offline: bool, info: Package) -> Result<()> {
    let deps = if matches.is_present("test") {
        info.all_dependencies().context(Kind::InvalidElmJson)?
    } else {
        info.dependencies()
    };

    let mut retriever =
        Retriever::new(&info.elm_version().to_constraint(), offline).context(Kind::Unknown)?;

    if matches.is_present("minimize") {
        retriever.minimize();
    }

    let extras = util::add_extra_deps(matches, &mut retriever);

    retriever.add_deps(deps.iter().filter(|(k, _)| !extras.contains(k)));

    Resolver::new(&mut retriever)
        .solve()
        .context(Kind::NoResolution)
        .and_then(|x| serde_json::to_string(&AppDependencies::from(x)).context(Kind::Unknown))
        .map(|v| println!("{}", v))?;
    Ok(())
}
