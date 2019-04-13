extern crate clap;
extern crate elm_json;
extern crate serde_json;
extern crate slog;
extern crate slog_async;
extern crate slog_term;

use clap::{App, Arg, ArgMatches, SubCommand};
use elm_json::package::{self, retriever::Retriever};
use elm_json::solver::{self, Resolver};
use failure::{bail, format_err, Error};
use petgraph::{self, visit::IntoNodeReferences};
use serde::Serialize;
use slog::{o, Drain, Logger};
use std::collections::BTreeMap;
use std::fs::File;
use std::io::BufReader;

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
            SubCommand::with_name("solve")
                .about("Figure out a solution given the version constraints in your elm.json")
                .arg(
                    Arg::with_name("INPUT")
                        .help("The elm.json file to solve")
                        .index(1)
                        .default_value("elm.json"),
                )
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
                ),
        )
        .get_matches();

    let min_log_level = match matches.occurrences_of("verbose") {
        0 => slog::Level::Error,
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
    } else {
        Ok(())
    }
}

fn solve(matches: &ArgMatches, logger: &Logger) -> Result<(), Error> {
    let path = matches.value_of("INPUT").unwrap();
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    let info: package::Package = serde_json::from_reader(reader)?;

    let deps = if matches.is_present("test") {
        info.all_dependencies()?
    } else {
        info.dependencies()
    };

    let mut retriever: Retriever = Retriever::new(&logger, &deps);
    retriever.fetch_versions()?;

    for dep in matches
        .values_of_lossy("extra")
        .unwrap_or_else(|| Vec::new())
        .iter()
    {
        let parts: Vec<&str> = dep.split('@').collect();
        match parts.as_slice() {
            [name] => retriever.add_dep(&name.to_string(), &None),
            [name, version] => {
                let version: package::Version = version.parse()?;
                retriever.add_dep(&name.to_string(), &Some(version))
            }
            _ => bail!("What"),
        }
    }

    let res = Resolver::new(&logger, &mut retriever)
        .solve()
        .and_then(|x| serde_json::to_string(&Deps::new(x)).map_err(|e| format_err!("{}", e)));
    match res {
        Ok(v) => println!("{}", v),
        Err(e) => println!("{}", e),
    }
    Ok(())
}

#[derive(Debug, Serialize)]
struct Deps {
    direct: BTreeMap<String, package::Version>,
    indirect: BTreeMap<String, package::Version>,
}

impl Deps {
    pub fn new(g: solver::Graph<solver::Summary<String>>) -> Deps {
        let mut direct: BTreeMap<String, package::Version> = BTreeMap::new();
        let mut indirect: BTreeMap<String, package::Version> = BTreeMap::new();
        let root = g.node_references().nth(0).unwrap().0;
        let mut bfs = petgraph::visit::Bfs::new(&g, root);

        while let Some(nx) = bfs.next(&g) {
            if nx == root {
                continue;
            }
            let item = g[nx].clone();
            if g.find_edge(root, nx).is_some() {
                direct.insert(item.id, item.version.into());
            } else {
                indirect.insert(item.id, item.version.into());
            }
        }

        Deps { direct, indirect }
    }
}
