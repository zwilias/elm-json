extern crate semver_constraints;
use semver_constraints::Version;

#[derive(Clone, Eq, PartialEq, Hash, Debug)]
pub struct Summary<PackageId: Clone + PartialEq + std::hash::Hash + Eq> {
    pub id: PackageId,
    pub version: Version,
}

impl<PackageId> Summary<PackageId>
where
    PackageId: Clone + PartialEq + Eq + std::hash::Hash,
{
    pub fn new(id: PackageId, version: Version) -> Self {
        Self { id, version }
    }

    pub fn version(&self) -> Version {
        self.version.clone()
    }

    pub fn id(&self) -> PackageId {
        self.id.clone()
    }
}
