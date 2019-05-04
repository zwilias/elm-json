use super::{util, ErrorKind, Result};
use crate::{
    diff,
    package::{
        self,
        retriever::{PackageId, Retriever},
    },
    project::{self, Application, Package, Project},
    semver,
    solver::Resolver,
};
use clap::ArgMatches;
use colored::Colorize;
use failure::ResultExt;
use petgraph::{self, visit::IntoNodeReferences};
use slog::Logger;
use std::collections::BTreeMap;

pub fn run(matches: &ArgMatches, logger: &Logger) -> Result<()> {
    util::with_elm_json(&matches, &logger, install_application, install_package)
}

fn install_package(matches: &ArgMatches, logger: &Logger, info: Package) -> Result<()> {
    let mut retriever =
        Retriever::new(&logger, &info.elm_version().to_constraint()).context(ErrorKind::Unknown)?;

    let deps = info.all_dependencies().context(ErrorKind::InvalidElmJson)?;
    retriever.add_deps(&deps);
    let extras = util::add_extra_deps(matches, &mut retriever);

    let res = Resolver::new(&logger, &mut retriever)
        .solve()
        .context(ErrorKind::NoResolution)?;

    let mut deps: BTreeMap<_, package::Range> = BTreeMap::new();
    let mut test_deps: BTreeMap<_, package::Range> = BTreeMap::new();
    let direct_dep_names: Vec<_> = info.dependencies.keys().cloned().collect();
    let root = res.node_references().nth(0).unwrap().0;
    let for_test = matches.is_present("test");

    for idx in res.neighbors(root) {
        let item = res[idx].clone();
        if let PackageId::Pkg(dep) = item.id {
            if extras.contains(&dep) {
                let r: package::Range = item.version.into();
                if for_test {
                    test_deps.insert(dep.clone(), r);
                } else {
                    deps.insert(dep.clone(), r);
                }
            } else if direct_dep_names.contains(&dep) {
                deps.insert(dep.clone(), info.dependencies[&dep]);
            } else {
                test_deps.insert(dep.clone(), info.test_dependencies[&dep]);
            }
        }
    }

    if info.dependencies == deps && info.test_dependencies == test_deps {
        println!("\n{}\n", util::format_header("NO CHANGES REQUIRED").green());
        println!("All the requested packages are already available!");
        std::process::exit(0);
    }

    println!(
        "\n{}\n",
        util::format_header("PACKAGE CHANGES READY").green()
    );

    diff::show(diff::Kind::Regular, &info.dependencies, &deps);
    diff::show(diff::Kind::Test, &info.test_dependencies, &test_deps);

    let updated = Project::Package(info.with_deps(deps, test_deps));

    if util::confirm("Should I make these changes?", &matches)? {
        util::write_elm_json(&updated, &matches)?;
        println!("Saved updated elm.json!");
    } else {
        println!("Aborting!");
    }

    Ok(())
}

fn install_application(matches: &ArgMatches, logger: &Logger, info: Application) -> Result<()> {
    let strictness = semver::Strictness::Exact;
    let elm_version = info.elm_version();

    let mut retriever: Retriever =
        Retriever::new(&logger, &elm_version.into()).context(ErrorKind::Unknown)?;

    let extras = util::add_extra_deps(matches, &mut retriever);

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

    let extra_direct: Vec<_> = if matches.is_present("test") {
        Vec::new()
    } else {
        extras.iter().cloned().collect()
    };

    let mut orig_direct = info
        .dependencies
        .direct
        .keys()
        .filter(|&x| !extras.contains(&x.clone()))
        .cloned()
        .collect::<Vec<_>>();
    orig_direct.extend(extra_direct);

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
