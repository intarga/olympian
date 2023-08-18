use thiserror::Error;

mod qc_tests;
pub use qc_tests::{
    buddy_check::buddy_check,
    dip_check::dip_check,
    sct::{sct, SctOutput},
    step_check::step_check,
};

mod util;
pub use util::spatial_tree::SpatialTree;
pub use util::Flag;

#[derive(Error, Debug, Clone)]
#[non_exhaustive]
pub enum Error {
    #[error("input vector {0} does not have compatible size")]
    InvalidInputShape(String),
    #[error("argument {0} does not have a valid value: {1}")]
    InvalidArg(String, String),
}
