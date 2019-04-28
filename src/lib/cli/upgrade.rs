use super::util;
use crate::{
    package::retriever::Retriever,
    project::{self, Application, Project},
    semver,
    solver::Resolver,
};
use clap::ArgMatches;
use colored::Colorize;
use dialoguer::Confirmation;
use failure::Error;
use slog::Logger;

pub fn run(matches: &ArgMatches, logger: &Logger) -> Result<(), Error> {
    util::with_elm_json(&matches, &logger, upgrade_application, |_, _, _| {
        util::unsupported("Upgrading dependencies for packages is not yet supported.")
    })
}
fn upgrade_application(
    matches: &ArgMatches,
    logger: &Logger,
    info: &Application,
) -> Result<(), Error> {
    let strictness = if matches.is_present("unsafe") {
        semver::Strictness::Unsafe
    } else {
        semver::Strictness::Safe
    };
    let elm_version = info.elm_version();

    let mut retriever: Retriever = Retriever::new(&logger, &elm_version.into())?;

    retriever.add_deps(&info.dependencies(&strictness));
    retriever.add_deps(&info.test_dependencies(&strictness));

    let res = Resolver::new(&logger, &mut retriever)
        .solve()
        .unwrap_or_else(|e| {
            util::error_out("NO VALID PACKAGE VERSIONS FOUND", &e);
            unreachable!()
        });

    let direct_deps: Vec<_> = info.dependencies.direct.keys().cloned().collect();
    let deps = project::reconstruct(&direct_deps, &res);

    if deps.0 == info.dependencies {
        println!("\n{}\n", util::format_header("PACKAGES UP TO DATE").green());
        println!("All your dependencies appear to be up to date!");
        return Ok(());
    }

    println!(
        "\n{}\n",
        util::format_header("PACKAGE UPGRADES FOUND").green()
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
