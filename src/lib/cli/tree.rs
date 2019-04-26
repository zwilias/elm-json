use super::util;
use crate::{
    package::{
        retriever::{self, Retriever},
        Package,
    },
    project::{Application, Project},
    semver,
    solver::{self, Resolver},
};
use clap::ArgMatches;
use colored::Colorize;
use failure::Error;
use itertools::Itertools;
use petgraph::{self, visit::IntoNodeReferences};
use slog::Logger;
use std::collections::HashSet;

pub fn run(matches: &ArgMatches, logger: &Logger) -> Result<(), Error> {
    match util::read_elm_json(&matches)? {
        Project::Application(app) => tree_application(&matches, &logger, &app),
        Project::Package(pkg) => tree_package(&matches, &logger, &pkg),
    }
}

fn tree_application(
    matches: &ArgMatches,
    logger: &Logger,
    info: &Application,
) -> Result<(), Error> {
    let mut deps: Vec<_> = info.dependencies(&semver::Strictness::Exact);
    let indirect = &info.indirect_dependencies();
    let elm_version = info.elm_version();

    let mut retriever: Retriever = Retriever::new(&logger, &elm_version.into())?;

    retriever.add_preferred_versions(
        indirect
            .iter()
            .map(|(k, v)| (k.clone().into(), *v))
            .collect(),
    );

    if matches.is_present("test") {
        deps.extend(info.test_dependencies(&semver::Strictness::Exact));

        retriever.add_preferred_versions(
            info.indirect_test_dependencies()
                .into_iter()
                .map(|(k, v)| (k.into(), v))
                .collect(),
        )
    }

    retriever.add_deps(&deps);

    let res = Resolver::new(&logger, &mut retriever).solve();
    match res {
        Ok(v) => show_tree(&v),
        Err(e) => util::error_out("NO VALID PACKAGE VERSIONS FOUND", &e),
    }
    Ok(())
}

fn tree_package(matches: &ArgMatches, logger: &Logger, info: &Package) -> Result<(), Error> {
    let deps = if matches.is_present("test") {
        info.all_dependencies()?
    } else {
        info.dependencies()
    };

    let mut retriever = Retriever::new(&logger, &info.elm_version().to_constraint())?;
    retriever.add_deps(&deps);

    let res = Resolver::new(&logger, &mut retriever).solve();
    match res {
        Ok(v) => show_tree(&v),
        Err(e) => util::error_out("NO VALID PACKAGE VERSIONS FOUND", &e),
    }
    Ok(())
}

fn show_tree(g: &solver::Graph<solver::Summary<retriever::PackageId>>) {
    let root = g.node_references().nth(0).unwrap().0;
    let mut visited: HashSet<usize> = HashSet::new();
    println!("\nproject");

    visit_children("", &g, &mut visited, root);

    println!("\nItems marked with {} have their dependencies ommitted since they've already\nappeared in the output.", "*".blue());
}

fn visit_children(
    prefix: &str,
    g: &solver::Graph<solver::Summary<retriever::PackageId>>,
    mut visited: &mut HashSet<usize>,
    root: petgraph::graph::NodeIndex,
) {
    let mut graph_iter = g
        .neighbors_directed(root, petgraph::Direction::Outgoing)
        .filter(|&idx| {
            if let retriever::PackageId::Pkg(_) = &g[idx].id {
                true
            } else {
                false
            }
        })
        .sorted_by(|&a, &b| Ord::cmp(&g[a].id, &g[b].id))
        .peekable();

    while let Some(idx) = graph_iter.next() {
        let item = &g[idx];
        let repeated = visited.contains(&idx.index());
        visited.insert(idx.index());

        if let retriever::PackageId::Pkg(name) = &item.id {
            let (s, e) = if graph_iter.peek().is_some() {
                ("\u{251c}\u{2500}\u{2500}", "\u{2502}   ")
            } else {
                ("\u{2514}\u{2500}\u{2500}", "    ")
            };
            println!(
                "{}{} {} @ {}{}",
                prefix,
                s,
                name,
                item.version,
                if repeated { " *".blue() } else { "".clear() }
            );

            if !repeated {
                visit_children(&(prefix.to_owned() + e), &g, &mut visited, idx)
            }
        }
    }
}
