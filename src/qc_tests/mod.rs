// use std::{error::Error, fmt::Display};
use thiserror::Error;

pub(super) mod buddy_check;
pub(super) mod dip_check;
pub(super) mod sct;
pub(super) mod step_check;

#[derive(Error, Debug)]
#[non_exhaustive]
pub enum Error {
    #[error("input vector {0} does not have compatible size")]
    InvalidInputShape(String),
    #[error("argument {0} does not have a valid value: {1}")]
    InvalidArg(String, String),
}
