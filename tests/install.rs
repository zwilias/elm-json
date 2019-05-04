use assert_cmd::prelude::*;
use assert_fs::prelude::*;
use predicates::prelude::*;
use std::path::Path;
use std::{error::Error, process::Command};

fn elm_json_install() -> Result<Command, Box<Error>> {
    let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME"))?;
    cmd.arg("install");
    Ok(cmd)
}

type TestResult = Result<(), Box<Error>>;

#[test]
fn no_elm_json() -> TestResult {
    let mut cmd = elm_json_install()?;
    cmd.arg("elm/core").arg("--").arg("foo/elm.json");

    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("MISSING ELM.JSON"));

    Ok(())
}

#[test]
fn install_on_package_succeeds() -> TestResult {
    let temp = assert_fs::TempDir::new()?;
    temp.child("elm.json")
        .write_file(Path::new("tests/fixtures/empty_package.elm.json"))?;

    let mut cmd = elm_json_install()?;
    cmd.current_dir(temp.path()).arg("--yes").arg("elm/core");
    cmd.assert().success();

    temp.child("elm.json")
        .assert(predicate::str::contains("elm/core"));

    Ok(())
}

#[test]
fn install_on_application_succeeds() -> TestResult {
    let temp = assert_fs::TempDir::new()?;
    temp.child("elm.json")
        .write_file(Path::new("tests/fixtures/empty_application.elm.json"))?;

    let mut cmd = elm_json_install()?;
    cmd.current_dir(temp.path()).arg("--yes").arg("elm/core");
    cmd.assert().success();

    temp.child("elm.json")
        .assert(predicate::str::contains("elm/core"));

    Ok(())
}
