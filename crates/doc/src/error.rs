//! Error type of the design model.

use std::fmt;

use crate::ids::{ItemId, SheetId};

/// Why an operation was refused or a project is inconsistent. A refused operation leaves the
/// project unchanged.
#[derive(Debug, Clone, PartialEq)]
pub enum DocError {
    /// No sheet has this id.
    NoSuchSheet(SheetId),
    /// No item has this id.
    NoSuchItem(ItemId),
    /// A sheet with this id already exists.
    DuplicateSheet(SheetId),
    /// An item with this id already exists.
    DuplicateItem(ItemId),
    /// A coordinate, offset or size is NaN or infinite (`what` names it).
    NonFinite(&'static str),
    /// An item is malformed (for example a wire with fewer than two points).
    Invalid(String),
    /// The project-wide id counter is used up.
    IdsExhausted,
    /// Batches are nested deeper than [`MAX_BATCH_DEPTH`](crate::MAX_BATCH_DEPTH).
    BatchTooDeep,
}

impl fmt::Display for DocError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoSuchSheet(id) => write!(f, "there is no {id}"),
            Self::NoSuchItem(id) => write!(f, "there is no {id}"),
            Self::DuplicateSheet(id) => write!(f, "{id} already exists"),
            Self::DuplicateItem(id) => write!(f, "{id} already exists"),
            Self::NonFinite(what) => write!(f, "{what} is not a finite number"),
            Self::Invalid(why) => f.write_str(why),
            Self::IdsExhausted => f.write_str("the project has run out of ids"),
            Self::BatchTooDeep => write!(f, "operation batches are nested deeper than {}", crate::MAX_BATCH_DEPTH),
        }
    }
}

impl std::error::Error for DocError {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::WireId;

    #[test]
    fn messages_name_the_item() {
        assert_eq!(DocError::NoSuchItem(ItemId::Wire(WireId(4))).to_string(), "there is no wire#4");
        assert_eq!(DocError::NonFinite("wire point").to_string(), "wire point is not a finite number");
        assert!(DocError::BatchTooDeep.to_string().contains("64"));
    }
}
