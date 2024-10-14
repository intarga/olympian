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
    aggregate_less_than_set::aggregate_less_than_set,
    buddy_check::buddy_check,
    flatline_check::{flatline_check, flatline_check_cache},
    range_check::{range_check, range_check_cache},
    range_check_humidity::range_check_humidity,
    range_check_wind_direction::range_check_wind_direction,
    sct::sct,
    special_values_check::{special_values_check, special_values_check_cache},
    spike_check::{spike_check, spike_check_cache, SPIKE_LEADING_PER_RUN, SPIKE_TRAILING_PER_RUN},
    step_check::{step_check, step_check_cache, STEP_LEADING_PER_RUN},
};

mod util;
pub use util::DataCache;
pub use util::Flag;

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
