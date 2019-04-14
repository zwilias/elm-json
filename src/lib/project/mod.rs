use crate::semver::{Range, Strictness, Version};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

pub use crate::package::Package;

#[derive(Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum Project {
    Application(Application),
    Package(Package),
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct Application {
    elm_version: Version,
    dependencies: AppDependencies,
    test_dependencies: AppDependencies,
    #[serde(flatten)]
    other: HashMap<String, Value>,
}

#[derive(Serialize, Deserialize)]
pub struct AppDependencies {
    direct: HashMap<String, Version>,
    indirect: HashMap<String, Version>,
}

impl Application {
    pub fn dependencies(&self, strictness: &Strictness) -> Vec<(String, Range)> {
        self.dependencies
            .direct
            .iter()
            .map(|(k, &v)| (k.clone(), Range::from(&v, &strictness)))
            .collect()
    }

    pub fn indirect_dependencies(&self) -> HashMap<String, Version> {
        self.dependencies.indirect.clone()
    }

    pub fn elm_version(&self) -> Version {
        self.elm_version
    }
}
