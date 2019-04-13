use super::{incompat::Incompatibility, summary::Summary};
use failure::Error;
use semver_constraints::{Constraint, Version};

pub trait Retriever {
    type PackageId: std::hash::Hash + Eq + Clone + std::fmt::Display;

    fn root(&self) -> Summary<Self::PackageId>;
    fn incompats(
        &mut self,
        pkg: &Summary<Self::PackageId>,
    ) -> Result<Vec<Incompatibility<Self::PackageId>>, Error>;
    fn count_versions(&self, pkg: &Self::PackageId) -> usize;
    fn best(&mut self, pkg: &Self::PackageId, con: &Constraint) -> Result<Version, Error>;
}
