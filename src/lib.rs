//! Quality control routines for meteorological data.
//!
//! In addition to the routines themselves, this crate also provides a [`Flag`] type, as well as
//! [`SeriesCache`] and [`SpatialCache`] as standard formats for data to be fed into timeseries
//! and spatial QC tests respectively.
//!
//! ```
//! use olympian::{checks::spatial::{buddy_check, BuddyCheckArgs}, Flag, SpatialTree, SingleOrVec};
//!
//! assert_eq!(
//!     buddy_check(
//!         &[Some(0.), Some(0.), Some(1.)],
//!         &SpatialTree::from_latlons(
//!             [60., 60., 60.].to_vec(),
//!             [60., 60.00011111, 60.00022222].to_vec(),
//!             [0., 0., 0.].to_vec(),
//!         ),
//!         &BuddyCheckArgs {
//!             radii: SingleOrVec::Single(10000.),
//!             nums_min: SingleOrVec::Single(1),
//!             threshold: 1.,
//!             max_elev_diff: 200.,
//!             elev_gradient: -0.0065,
//!             min_std: 0.01,
//!             num_iterations: 2,
//!         },
//!         None,
//!     )
//!     .unwrap(),
//!     [Flag::Pass, Flag::Pass, Flag::Fail]
//! )
//! ```

#![warn(missing_docs)]
#![warn(missing_debug_implementations)]

use thiserror::Error;

/// Algorithms that can be used to QC meteorological data.
pub mod checks;

mod util;
pub use util::{spatial_tree::SpatialTree, DataCache, Flag, SingleOrVec};

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
