// #![deny(missing_docs)]

use thiserror::Error;

mod qc_tests;
pub use qc_tests::{
    buddy_check::buddy_check, dip_check::dip_check, sct::sct, step_check::step_check,
};

mod util;
pub use util::Flag;
pub use util::SeriesCache;
pub use util::SpatialCache;

/// Error type for Olympian
#[derive(Error, Debug, Clone)]
#[non_exhaustive]
pub enum Error {
    /// The shape of an input value is not valid
    #[error("input vector {0} does not have compatible size")]
    InvalidInputShape(String),
    /// An argument has an invalid value
    #[error("argument {0} does not have a valid value: {1}")]
    InvalidArg(String, String),
}
