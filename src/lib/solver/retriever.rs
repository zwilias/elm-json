use super::{incompat::Incompatibility, summary};
use crate::semver::{Constraint, Version};
use failure::Error;

pub trait Retriever {
    type PackageId: summary::PackageId;

    fn root(&self) -> summary::Summary<Self::PackageId>;
    fn incompats(
        &mut self,
        pkg: &summary::Summary<Self::PackageId>,
    ) -> Result<Vec<Incompatibility<Self::PackageId>>, Error>;
    fn count_versions(&self, pkg: &Self::PackageId) -> usize;
    fn best(&mut self, pkg: &Self::PackageId, con: &Constraint) -> Result<Version, Error>;
}
