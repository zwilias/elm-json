use clap::{App, AppSettings, Arg, SubCommand};

pub mod completions;
pub mod error;
pub mod install;
pub mod new;
pub mod solve;
pub mod tree;
pub mod uninstall;
pub mod upgrade;
pub mod util;

pub use error::Kind;

pub fn build() -> App<'static, 'static> {
    App::new("elm-json")
        .version(env!("CARGO_PKG_VERSION"))
        .about("Deal with your elm.json")
        .setting(AppSettings::SubcommandRequiredElseHelp)
        .max_term_width(80)
        .arg(
            Arg::with_name("verbose")
                .short("v")
                .long("verbose")
                .multiple(true)
                .help("Sets the level of verbosity"),
        )
        .arg(
            Arg::with_name("offline")
                .long("offline")
                .multiple(false)
                .help("Enable offline mode, which means no HTTP traffic will happen"),
        )
        .subcommand(
            SubCommand::with_name("upgrade")
                .about("Bring your dependencies up to date")
                .arg(
                    Arg::with_name("unsafe")
                        .help("Allow major versions bumps")
                        .long("unsafe"),
                )
                .arg(Arg::with_name("yes")
                     .help("Answer \"yes\" to all questions")
                     .long("yes")
                )
                .arg(
                    Arg::with_name("INPUT")
                        .help("The elm.json file to upgrade")
                        .default_value("elm.json"),
                ),
        )
        .subcommand(
            SubCommand::with_name("install")
                .about("Install a package")
                .arg(
                    Arg::with_name("test")
                        .help("Install as a test-dependency")
                        .long("test"),
                )
                .arg(Arg::with_name("yes")
                     .help("Answer \"yes\" to all questions")
                     .long("yes")
                )
                .arg(
                    Arg::with_name("extra")
                        .help("Package to install, e.g. elm/core or elm/core@1.0.2 or elm/core@1")
                        .takes_value(true)
                        .value_name("PACKAGE")
                        .validator(util::valid_package)
                        .required(true)
                        .multiple(true),
                )
                .arg(
                    Arg::with_name("INPUT")
                        .help("The elm.json file to upgrade")
                        .last(true)
                        .default_value("elm.json"),
                ),
        )
        .subcommand(
            SubCommand::with_name("uninstall")
                .about("Uninstall a package")
                .arg(Arg::with_name("yes")
                     .help("Answer \"yes\" to all questions")
                     .long("yes")
                )
                .arg(
                    Arg::with_name("extra")
                        .help("Package to uninstall, e.g. elm/html")
                        .takes_value(true)
                        .value_name("PACKAGE")
                        .required(true)
                        .validator(util::valid_package_name)
                        .multiple(true),
                )
                .arg(
                    Arg::with_name("INPUT")
                        .help("The elm.json file to upgrade")
                        .last(true)
                        .default_value("elm.json"),
                ),
        )
        .subcommand(
            SubCommand::with_name("tree")
                .about("List entire dependency graph as a tree")
                .arg(
                    Arg::with_name("test")
                        .help("Promote test-dependencies to top-level dependencies")
                        .long("test"),
                )
                .arg(
                    Arg::with_name("package")
                        .help("Limit output to show path to some (indirect) dependency")
                        .takes_value(true)
                        .value_name("PACKAGE")
                        .validator(util::valid_package),
                )
                .arg(
                    Arg::with_name("INPUT")
                        .help("The elm.json file to solve")
                        .last(true)
                        .default_value("elm.json"),
                ),
        )
        .subcommand(
            SubCommand::with_name("solve")
                .about("Figure out a solution given the version constraints in your elm.json")
                .long_about("This is mostly useful for tooling wishing to consume the elm.json with particular constraints.\n\nIt could be used to get a concrete set of packages that match the constraints set by the elm.json of a package, or to find the minimal versions needed for consuming a package. The --test flag also adds test-dependencies into the mix. This command - when succesfull - writes some JSON to stdout which should be formatted in a way to be valid for use as the `dependencies` key in an application")
                .setting(AppSettings::Hidden)
                .arg(
                    Arg::with_name("test")
                        .help("Promote test-dependencies to top-level dependencies")
                        .long("test"),
                )
                .arg(
                    Arg::with_name("minimize")
                        .help("Choose lowest available versions rather than highest")
                        .short("m")
                        .long("minimize"),
                )
                .arg(
                    Arg::with_name("extra")
                        .short("e")
                        .long("extra")
                        .help("Specify extra dependencies, e.g. elm/core or elm/core@1.0.2")
                        .takes_value(true)
                        .value_name("PACKAGE")
                        .validator(util::valid_package)
                        .multiple(true),
                )
                .arg(
                    Arg::with_name("INPUT")
                        .help("The elm.json file to solve")
                        .default_value("elm.json"),
                ),
        )
        .subcommand(
            SubCommand::with_name("completions")
                .about("Generates completion scripts for your shell")
                .setting(AppSettings::Hidden)
                .arg(
                    Arg::with_name("SHELL")
                        .required(true)
                        .possible_values(&["bash", "fish", "zsh"])
                        .help("The shell to generate the script for")
                )
        )
        .subcommand(SubCommand::with_name("new").about("Create a new elm.json file"))
}
