use crate::{
    package::retriever::{PackageId, Retriever},
    semver,
    semver::Version,
    solver,
};
use clap::ArgMatches;
use colored::Colorize;
use failure::Error;
use std::collections::{BTreeMap, HashSet};

pub fn find_by_name(name: &str, g: &solver::Graph<solver::Summary<PackageId>>) -> Option<Version> {
    for idx in g.node_indices() {
        let item = g[idx].clone();
        match item.id {
            PackageId::Pkg(n) => {
                if n == name {
                    return Some(g[idx].version);
                }
                continue;
            }
            _ => continue,
        }
    }
    None
}

pub fn add_extra_deps(
    matches: &ArgMatches,
    retriever: &mut Retriever,
) -> Result<HashSet<String>, Error> {
    let mut extras = HashSet::new();

    for dep in matches
        .values_of_lossy("extra")
        .unwrap_or_else(Vec::new)
        .iter()
    {
        let parts: Vec<&str> = dep.split('@').collect();
        match parts.as_slice() {
            [name] => {
                retriever.add_dep(&name.to_string(), &None);
                extras.insert(name.to_string());
            }
            [name, version] => {
                let version: semver::Version = version.parse()?;
                retriever.add_dep(&name.to_string(), &Some(version));
                extras.insert(name.to_string());
            }
            _ => unreachable!(),
        }
    }
    Ok(extras)
}

pub fn error_out(msg: &str, e: Error) {
    println!("\n{}", format_header(msg).cyan());
    println!("\n{}", textwrap::fill(&e.to_string(), 80));
    std::process::exit(1)
}

pub fn format_header(x: &str) -> String {
    format!("-- {} {}", x, "-".repeat(80 - 4 - x.len()))
}

pub fn unsupported(description: &str) -> Result<(), Error> {
    println!("\n{}\n", format_header("COMMAND NOT YET IMPLEMENTED").red());
    println!("{}", textwrap::fill(description, 80));
    std::process::exit(1)
}

pub fn show_diff<T>(title: &str, left: &BTreeMap<String, T>, right: &BTreeMap<String, T>)
where
    T: Eq + std::fmt::Display + Sized + Copy,
{
    let it = Diff::new(&left, &right);
    if !it.is_empty() {
        println!(
            "I want to make some changes to your {} dependencies\n",
            title.bold()
        );
        it.print();
        println!();
    }
}

impl<T> Diff<T>
where
    T: Sized + Eq + Copy + std::fmt::Display,
{
    pub fn new(left: &BTreeMap<String, T>, right: &BTreeMap<String, T>) -> Diff<T> {
        let mut only_left = Vec::new();
        let mut only_right = Vec::new();
        let mut changed = Vec::new();

        let mut iter_left = left.iter();
        let mut iter_right = right.iter();

        let mut left = iter_left.next();
        let mut right = iter_right.next();

        while let (Some((left_name, left_version)), Some((right_name, right_version))) =
            (left, right)
        {
            if left_name == right_name {
                if left_version != right_version {
                    changed.push((left_name.clone(), *left_version, *right_version))
                }

                left = iter_left.next();
                right = iter_right.next();
                continue;
            }

            if left_name < right_name {
                only_left.push((left_name.clone(), *left_version));
                left = iter_left.next();
                continue;
            }

            if left_name > right_name {
                only_right.push((right_name.clone(), *right_version));
                right = iter_right.next();
                continue;
            }
        }

        while let Some((name, version)) = left {
            only_left.push((name.clone(), *version));
            left = iter_left.next();
        }

        while let Some((name, version)) = right {
            only_right.push((name.clone(), *version));
            right = iter_right.next();
        }

        Diff {
            only_left,
            only_right,
            changed,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.only_left.is_empty() && self.only_right.is_empty() && self.changed.is_empty()
    }

    pub fn print(&self) {
        for (k, v) in self.only_left.iter() {
            println!("- {} {} {}", "[DEL]".yellow(), k, v);
        }

        for (k, o, n) in self.changed.iter() {
            println!("- {} {} {} -> {}", "[CHG]".blue(), k, o, n);
        }

        for (k, v) in self.only_right.iter() {
            println!("- {} {} {}", "[ADD]".green(), k, v);
        }
    }
}

pub struct Diff<T>
where
    T: Eq + Sized + Copy + std::fmt::Display,
{
    only_left: Vec<(String, T)>,
    only_right: Vec<(String, T)>,
    changed: Vec<(String, T, T)>,
}
