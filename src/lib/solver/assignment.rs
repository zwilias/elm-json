/* COPYRIGHT (C) 2018 DAVID CAO

PERMISSION IS HEREBY GRANTED, FREE OF CHARGE, TO ANY PERSON OBTAINING A COPY
OF THIS SOFTWARE AND ASSOCIATED DOCUMENTATION FILES (THE "SOFTWARE"), TO DEAL
IN THE SOFTWARE WITHOUT RESTRICTION, INCLUDING WITHOUT LIMITATION THE RIGHTS
TO USE, COPY, MODIFY, MERGE, PUBLISH, DISTRIBUTE, SUBLICENSE, AND/OR SELL
COPIES OF THE SOFTWARE, AND TO PERMIT PERSONS TO WHOM THE SOFTWARE IS
FURNISHED TO DO SO, SUBJECT TO THE FOLLOWING CONDITIONS:

THE ABOVE COPYRIGHT NOTICE AND THIS PERMISSION NOTICE SHALL BE INCLUDED IN ALL
COPIES OR SUBSTANTIAL PORTIONS OF THE SOFTWARE.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
*/
//! Assignments for the dependency resolver.

use semver_constraints::{Constraint, Version};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Assignment<PackageId> {
    pub step: u16,
    pub level: u16,
    pub ty: AssignmentType,
    pub pkg: PackageId,
}

impl<PackageId> Assignment<PackageId> {
    pub fn new(step: u16, level: u16, pkg: PackageId, ty: AssignmentType) -> Self {
        Assignment {
            step,
            level,
            ty,
            pkg,
        }
    }

    pub fn ty(&self) -> &AssignmentType {
        &self.ty
    }

    pub fn pkg(&self) -> &PackageId {
        &self.pkg
    }

    pub fn step(&self) -> u16 {
        self.step
    }

    pub fn level(&self) -> u16 {
        self.level
    }

    pub fn cause(&self) -> Option<usize> {
        match &self.ty {
            AssignmentType::Decision { version: _version } => None,
            AssignmentType::Derivation {
                cause,
                constraint: _constraint,
                positive: _positive,
            } => Some(*cause),
        }
    }

    pub fn constraint(&self) -> Constraint {
        match &self.ty {
            AssignmentType::Decision { version } => version.clone().into(),
            AssignmentType::Derivation {
                constraint,
                cause: _cause,
                positive: _positive,
            } => constraint.clone(),
        }
    }

    pub fn is_positive(&self) -> bool {
        match &self.ty {
            AssignmentType::Decision { version: _version } => false,
            AssignmentType::Derivation {
                positive,
                constraint: _constraint,
                cause: _cause,
            } => *positive,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AssignmentType {
    Decision {
        version: Version,
    },
    Derivation {
        constraint: Constraint,
        cause: usize,
        positive: bool,
    },
}
