//! Quality control routines for meteorological data.
//!
//! In addition to the routines themselves, this crate also provides a [`Flag`] type, as well as
//! [`SeriesCache`] and [`SpatialCache`] as standard formats for data to be fed into timeseries
//! and spatial QC tests respectively.
//!
//! ```
//! use olympian::{buddy_check, Flag, SpatialCache};
//!
//! assert_eq!(
//!     buddy_check(
//!         &SpatialCache::new(
//!             [60., 60., 60.].to_vec(),
//!             [60., 60.00011111, 60.00022222].to_vec(),
//!             [0., 0., 0.].to_vec(),
//!             [0., 0., 1.].to_vec()
//!         ),
//!         &[10000.],
//!         &[1],
//!         1.,
//!         200.,
//!         -0.0065,
//!         0.01,
//!         2,
//!         None,
//!     )
//!     .unwrap(),
//!     [Flag::Pass, Flag::Pass, Flag::Fail]
//! )
//! ```

#![warn(missing_docs)]
#![warn(missing_debug_implementations)]

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
