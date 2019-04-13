extern crate elm_json;
extern crate serde_json;
extern crate slog;
extern crate slog_async;
extern crate slog_term;

use elm_json::package::{self, retriever::Retriever};
use elm_json::solver::Resolver;
use failure::Error;
use slog::{o, Drain};
use std::fs::File;
use std::io::Read;

fn main() -> Result<(), Error> {
    let mut file = File::open("elm.json")?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;

    let info: package::Package = serde_json::from_str(&contents)?;

    let decorator = slog_term::PlainSyncDecorator::new(std::io::stdout());
    let drain = slog_term::FullFormat::new(decorator).build().fuse();
    let logger = slog::Logger::root(drain, o!());
    let mut retriever: Retriever = Retriever::new(&logger, &info.dependencies());
    retriever.fetch_versions()?;

    let res = Resolver::new(&logger, &mut retriever).solve();
    match res {
        Ok(v) => println!("Solved: {:#?}", v),
        Err(e) => println!("{}", e),
    }
    Ok(())
}
