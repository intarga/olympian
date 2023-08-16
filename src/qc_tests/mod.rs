use std::{error::Error, fmt::Display};

pub(super) mod buddy_check;
pub(super) mod dip_check;
pub(super) mod sct;
pub(super) mod step_check;

// TODO: use thiserror
#[derive(Debug)]
pub enum QcError {
    InvalidInputShape(String),
    InvalidArg((String, String)),
}

impl Display for QcError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Self::InvalidInputShape(cause) => {
                write!(f, "input vector {} does not have compatible size", cause)
            }
            Self::InvalidArg((argname, reason)) => {
                write!(
                    f,
                    "argument {} does not have a valid value: {}",
                    argname, reason
                )
            }
        }
    }
}

impl Error for QcError {}
