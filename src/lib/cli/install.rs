use super::util;
use crate::{
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
use dialoguer::Confirmation;
use failure::Error;
use petgraph::{self, visit::IntoNodeReferences};
use serde::ser::Serialize;
use slog::Logger;
use std::{
    collections::BTreeMap,
    fs::File,
    io::{BufReader, BufWriter},
};

pub fn run(matches: &ArgMatches, logger: &Logger) -> Result<(), Error> {
    let path = matches.value_of("INPUT").unwrap();
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    let info: Project = serde_json::from_reader(reader)?;

    match info {
        Project::Application(app) => install_application(&matches, &logger, &app),
        Project::Package(pkg) => install_package(&matches, &logger, &pkg),
    }
}

fn install_package(matches: &ArgMatches, logger: &Logger, info: &Package) -> Result<(), Error> {
    let mut retriever = Retriever::new(&logger, info.elm_version().to_constraint())?;

    let deps = info.all_dependencies()?;
    retriever.add_deps(&deps);
    let extras = util::add_extra_deps(matches, &mut retriever)?;

    let res = Resolver::new(&logger, &mut retriever)
        .solve()
        .unwrap_or_else(|e| {
            util::error_out("NO VALID PACKAGE VERSIONS FOUND", e);
            unreachable!()
        });

    let mut deps: BTreeMap<_, package::Range> = BTreeMap::new();
    let mut test_deps: BTreeMap<_, package::Range> = BTreeMap::new();
    let direct_dep_names: Vec<_> = info.dependencies.keys().cloned().collect();
    let root = res.node_references().nth(0).unwrap().0;
    let for_test = matches.is_present("test");

    for idx in res.neighbors(root) {
        let item = res[idx].clone();
        if let PackageId::Pkg(dep) = item.id {
            if extras.contains(&dep) {
                let r: package::Range = util::find_by_name(&dep, &res).unwrap().into();
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

    util::show_diff("", &info.dependencies, &deps);
    util::show_diff("test", &info.test_dependencies, &test_deps);

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
        let val = Project::Package(info.with_deps(deps, test_deps));
        val.serialize(&mut serializer)?;

        println!("Saved updated elm.json!");
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

    let mut retriever: Retriever = Retriever::new(&logger, elm_version.into())?;

    let extras = util::add_extra_deps(matches, &mut retriever)?;

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
