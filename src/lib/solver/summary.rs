use crate::semver::Version;
use std::fmt;

#[derive(Clone, Eq, PartialEq, Hash, Debug)]
pub struct Summary<P: PackageId> {
    pub id: P,
    pub version: Version,
}

pub trait PackageId: Clone + PartialEq + std::hash::Hash + Eq + fmt::Display {
    fn is_root(&self) -> bool;
}

impl<P> Summary<P>
where
    P: PackageId,
{
    pub fn new(id: P, version: Version) -> Self {
        Self { id, version }
    }

    pub fn version(&self) -> Version {
        self.version
    }

    pub fn id(&self) -> P {
        self.id.clone()
    }

    pub fn is_root(&self) -> bool {
        self.id.is_root()
    }
}
