use crate::{
    package::{self, retriever},
    semver::{Range, Strictness, Version},
    solver,
};
use petgraph::{self, visit::IntoNodeReferences};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, HashSet};

pub use crate::package::Package;

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum Project {
    Application(Application),
    Package(Package),
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct Application {
    source_directories: Vec<String>,
    elm_version: Version,
    pub dependencies: AppDependencies,
    pub test_dependencies: AppDependencies,
    #[serde(flatten)]
    other: BTreeMap<String, Value>,
}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct AppDependencies {
    pub direct: BTreeMap<package::Name, Version>,
    pub indirect: BTreeMap<package::Name, Version>,
}

pub fn reconstruct(
    direct_names: &[package::Name],
    g: &solver::Graph<solver::Summary<retriever::PackageId>>,
) -> (AppDependencies, AppDependencies) {
    let mut direct = BTreeMap::new();
    let mut indirect = BTreeMap::new();
    let mut test_direct = BTreeMap::new();
    let mut test_indirect = BTreeMap::new();
    let mut visited: HashSet<usize> = HashSet::new();
    let mut test_idxs: Vec<usize> = Vec::new();

    let root = g.node_references().nth(0).unwrap().0;

    for idx in g.neighbors(root) {
        let item = g[idx].clone();
        visited.insert(idx.index());

        if let retriever::PackageId::Pkg(name) = item.id {
            if direct_names.contains(&name) {
                direct.insert(name, item.version);
                let mut dfs = petgraph::visit::Dfs::new(&g, idx);
                while let Some(nx) = dfs.next(&g) {
                    if visited.contains(&nx.index()) {
                        continue;
                    }
                    visited.insert(nx.index());
                    let item = g[nx].clone();

                    if let retriever::PackageId::Pkg(dep) = item.id {
                        if direct_names.contains(&dep) {
                            continue;
                        }
                        indirect.insert(dep, item.version);
                    }
                }
            } else {
                test_idxs.push(idx.index());
            }
        }
    }

    for idx in test_idxs {
        let idx = petgraph::graph::NodeIndex::new(idx);
        let item = g[idx].clone();
        if let retriever::PackageId::Pkg(name) = item.id {
            test_direct.insert(name, item.version);

            let mut bfs = petgraph::visit::Bfs::new(&g, idx);
            while let Some(nx) = bfs.next(&g) {
                if visited.contains(&nx.index()) {
                    continue;
                }
                visited.insert(nx.index());
                let item = g[nx].clone();

                if let retriever::PackageId::Pkg(dep) = item.id {
                    test_indirect.insert(dep, item.version);
                }
            }
        }
    }

    (
        AppDependencies { direct, indirect },
        AppDependencies {
            direct: test_direct,
            indirect: test_indirect,
        },
    )
}

impl AppDependencies {
    pub fn new() -> Self {
        Self {
            direct: BTreeMap::new(),
            indirect: BTreeMap::new(),
        }
    }
}

impl From<solver::Graph<solver::Summary<retriever::PackageId>>> for AppDependencies {
    fn from(g: solver::Graph<solver::Summary<retriever::PackageId>>) -> Self {
        let mut direct: BTreeMap<package::Name, Version> = BTreeMap::new();
        let mut indirect: BTreeMap<package::Name, Version> = BTreeMap::new();
        let root = g.node_references().nth(0).unwrap().0;
        let mut bfs = petgraph::visit::Bfs::new(&g, root);

        while let Some(nx) = bfs.next(&g) {
            if nx == root {
                continue;
            }
            let item = g[nx].clone();

            if let retriever::PackageId::Pkg(name) = item.id {
                if g.find_edge(root, nx).is_some() {
                    direct.insert(name, item.version);
                } else {
                    indirect.insert(name, item.version);
                }
            }
        }

        Self { direct, indirect }
    }
}

impl Default for AppDependencies {
    fn default() -> Self {
        Self::new()
    }
}

impl Application {
    pub fn new() -> Self {
        let mut direct = BTreeMap::new();
        direct.insert(
            package::Name::new("elm", "core").unwrap(),
            Version::new(1, 0, 2),
        );
        let deps = AppDependencies {
            direct,
            indirect: BTreeMap::new(),
        };

        Self {
            source_directories: vec!["src".to_string()],
            elm_version: Version::new(0, 19, 0),
            dependencies: deps,
            test_dependencies: AppDependencies::new(),
            other: BTreeMap::new(),
        }
    }

    pub fn dependencies(&self, strictness: &Strictness) -> Vec<(package::Name, Range)> {
        self.dependencies
            .direct
            .iter()
            .map(|(k, &v)| (k.clone(), Range::from(&v, &strictness)))
            .collect()
    }

    pub fn test_dependencies(&self, strictness: &Strictness) -> Vec<(package::Name, Range)> {
        self.test_dependencies
            .direct
            .iter()
            .map(|(k, &v)| (k.clone(), Range::from(&v, &strictness)))
            .collect()
    }

    pub fn indirect_dependencies(&self) -> BTreeMap<package::Name, Version> {
        self.dependencies.indirect.clone()
    }

    pub fn indirect_test_dependencies(&self) -> BTreeMap<package::Name, Version> {
        self.test_dependencies.indirect.clone()
    }

    pub fn elm_version(&self) -> Version {
        self.elm_version
    }

    pub fn with_deps(&self, deps: AppDependencies) -> Self {
        Self {
            source_directories: self.source_directories.clone(),
            elm_version: self.elm_version,
            dependencies: deps,
            test_dependencies: self.test_dependencies.clone(),
            other: self.other.clone(),
        }
    }

    pub fn with_test_deps(&self, deps: AppDependencies) -> Self {
        Self {
            source_directories: self.source_directories.clone(),
            elm_version: self.elm_version,
            dependencies: self.dependencies.clone(),
            test_dependencies: deps,
            other: self.other.clone(),
        }
    }
}

impl Default for Application {
    fn default() -> Self {
        Self::new()
    }
}
