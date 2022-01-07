use super::{util, ErrorKind, Result};
use crate::{
    package::{
        self,
        retriever::{self, Retriever},
    },
    project::{Application, Package},
    semver,
    solver::{self, Resolver},
};
use clap::ArgMatches;
use colored::Colorize;
use failure::ResultExt;
use itertools::Itertools;
use petgraph::{self, visit::IntoNodeReferences};
use slog::Logger;
use std::collections::HashSet;
use std::iter::FromIterator;

pub fn run(matches: &ArgMatches, offline: bool, logger: &Logger) -> Result<()> {
    util::with_elm_json(matches, offline, logger, tree_application, tree_package)
}

fn tree_application(
    matches: &ArgMatches,
    offline: bool,
    logger: &Logger,
    info: Application,
) -> Result<()> {
    let mut deps: Vec<_> = info.dependencies(&semver::Strictness::Exact);
    let elm_version = info.elm_version();

    let mut retriever: Retriever =
        Retriever::new(logger, &elm_version.into(), offline).context(ErrorKind::Unknown)?;

    retriever.add_preferred_versions(
        info.dependencies
            .indirect
            .iter()
            .map(|(k, v)| (k.clone().into(), *v)),
    );

    if matches.is_present("test") {
        deps.extend(info.test_dependencies(&semver::Strictness::Exact));

        retriever.add_preferred_versions(
            info.test_dependencies
                .indirect
                .iter()
                .map(|(k, v)| (k.clone().into(), *v)),
        )
    }

    retriever.add_deps(&deps);

    Resolver::new(logger, &mut retriever)
        .solve()
        .map(|v| show_tree(&v, matches.value_of("package")))
        .context(ErrorKind::NoResolution)?;
    Ok(())
}

fn tree_package(matches: &ArgMatches, offline: bool, logger: &Logger, info: Package) -> Result<()> {
    let deps = if matches.is_present("test") {
        info.all_dependencies().context(ErrorKind::InvalidElmJson)?
    } else {
        info.dependencies()
    };

    let mut retriever = Retriever::new(logger, &info.elm_version().to_constraint(), offline)
        .context(ErrorKind::Unknown)?;
    retriever.add_deps(&deps);

    Resolver::new(logger, &mut retriever)
        .solve()
        .map(|v| show_tree(&v, matches.value_of("package")))
        .context(ErrorKind::NoResolution)?;
    Ok(())
}

fn show_tree(g: &solver::Graph<solver::Summary<retriever::PackageId>>, target: Option<&str>) {
    let root = g.node_references().next().unwrap().0;
    if let Some(target) = target {
        let name: package::Name = target.parse().expect("Invalid name parameter");

        if let Some(target) = g.node_indices().find(|i| g[*i].id.is(&name)) {
            let paths: Vec<Vec<_>> =
                petgraph::algo::all_simple_paths(g, root, target, 0, None).collect();
            let nodes: HashSet<_> = HashSet::from_iter(paths.concat());
            let mut g = g.clone();
            g.retain_nodes(|_, n| nodes.contains(&n));
            print_graph(&g, root)
        } else {
            println!(
                "Could not find {} in direct or indirect dependencies.",
                target
            )
        }
    } else {
        print_graph(g, root)
    }
}

fn print_graph(
    g: &solver::Graph<solver::Summary<retriever::PackageId>>,
    root: petgraph::graph::NodeIndex,
) {
    let mut visited: HashSet<usize> = HashSet::new();
    println!("\nproject");

    visit_children("", g, &mut visited, root);

    println!("\nItems marked with {} have their dependencies ommitted since they've already\nappeared in the output.", "*".blue());
}

fn visit_children(
    prefix: &str,
    g: &solver::Graph<solver::Summary<retriever::PackageId>>,
    visited: &mut HashSet<usize>,
    root: petgraph::graph::NodeIndex,
) {
    let mut graph_iter = g
        .neighbors_directed(root, petgraph::Direction::Outgoing)
        .filter(|&idx| matches!(&g[idx].id, retriever::PackageId::Pkg(_)))
        .sorted_by(|&a, &b| Ord::cmp(&g[a].id, &g[b].id))
        .peekable();

    while let Some(idx) = graph_iter.next() {
        let item = &g[idx];
        let repeated = visited.contains(&idx.index()) && g.edges(idx).next().is_some();
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
                visit_children(&(prefix.to_owned() + e), g, visited, idx)
            }
        }
    }
}
