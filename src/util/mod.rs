//! Utility types and functions for QC tests

pub mod spatial_tree;
use spatial_tree::SpatialTree;

use crate::Error;
use chronoutil::RelativeDuration;
use std::ops::Index;

/// Flag indicating result of a QC test for a given data point
#[derive(Copy, Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum Flag {
    /// The data point passed the QC test with no issues
    Pass,
    /// The data point failed the QC test
    Fail,
    /// The data point did not fail, but was inside a "warning" threshold
    Warn,
    /// The QC test was inconclusive
    Inconclusive,
    /// The input was invalid, so the data point could not be QCed
    Invalid,
    /// Some data needed for the test was missing
    ///
    /// This may have been the data point being QCed, or some other data that
    /// was needed to QC it. For example, a step check also needs the
    /// preceeding data point
    DataMissing,
    /// The data point did not have enough neighbours in the given radius
    ///
    /// Only relevant for spatial tests
    Isolated,
}

/// Unix timestamp, inner i64 is seconds since unix epoch
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Timestamp(pub i64);

/// A series of values representing a slice of a timeseries, tagged with an identifier of the
/// timeseries they are from.
#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub struct Timeseries<T> {
    /// String tag identifying the timeseries.
    pub tag: String,
    /// Data values in the timeseries.
    pub values: Vec<T>,
}

/// A pair of [`Timeseries`]
pub type TimeseriesPair<T> = (Timeseries<T>, Timeseries<T>);

/// Container for metereological data
///
/// a [`new`](DataCache::new) method is provided to
/// avoid the need to construct an R*-tree manually
#[derive(Debug, Clone)]
pub struct DataCache {
    /// Vector of [`Timeseries`].
    ///
    /// All of these timeseries are aligned on start_time and period.
    /// `None`s represent gaps in the series.
    ///
    /// Each timeseries vector can be represented as follows:
    /// |---|----------|---|
    /// where the first and last sections are `DataCache.num_leading_points` and
    /// `DataCache.num_trailing_points` long, respectively.
    /// The actual observations to be QCed (i.e. flagged) lie in the middle section.
    pub data: Vec<Timeseries<Option<f64>>>,
    /// Time of the first observation in data
    ///
    /// This means the first observation that will actually be QCed, so excluding the "leading"
    /// points.
    pub start_time: Timestamp,
    /// Period of the timeseries, i.e. the time gap between successive elements
    pub period: RelativeDuration,
    /// an [R*-tree](https://en.wikipedia.org/wiki/R*-tree) used to spatially
    /// index the data
    pub rtree: SpatialTree,
    /// The number of extra points in the series before the data to be QCed
    ///
    /// These points are needed because certain timeseries tests need more
    /// context around points to be able to QC them. The scheduler looks at
    /// the list of requested tests to figure out how many leading points will
    /// be needed, and requests a SeriesCache from the DataSwitch with that
    /// number of leading points
    pub num_leading_points: u8,
    /// The number of extra points in the series after the data to be QCed
    pub num_trailing_points: u8,
}

#[allow(clippy::too_many_arguments)]
impl DataCache {
    /// Create a new DataCache without manually constructing the R*-tree
    pub fn new(
        data: Vec<Timeseries<Option<f64>>>,
        lats: Vec<f64>,
        lons: Vec<f64>,
        elevs: Vec<f64>,
        start_time: Timestamp,
        period: RelativeDuration,
        num_leading_points: u8,
        num_trailing_points: u8,
    ) -> Self {
        // TODO: ensure vecs have same size
        Self {
            rtree: SpatialTree::from_latlons(lats, lons, elevs),
            data,
            start_time,
            period,
            num_leading_points,
            num_trailing_points,
        }
    }
}

/// Similar to [`DataCache`] but for use with consistency checks.
///
/// As consistency checks typically compare two timeseries of different parameters (and perhaps
/// different periods), the data field here contains a vector of pairs of timeseries which are
/// passed together into consistency checks.
///
/// The first timeseries in each pair should all have the same length as each other, same for the
/// second, and the length of the second should be `ratio *` the length of the first.
#[derive(Debug, Clone)]
pub struct ConsistencyCache {
    /// Vector of pairs of [`Timeseries`].
    ///
    /// Each pair of timeseries are aligned with the other pairs on start_time and period
    /// `None`s represent gaps in the series.
    pub data: Vec<TimeseriesPair<Option<f64>>>,
    /// Time of the first observation in the timeseries
    ///
    /// the first timestamp applies to the first timeseries in each pair, and so for the second
    pub start_time: (Timestamp, Timestamp),
    /// Period of the timeseries, i.e. the time gap between successive elements
    ///
    /// the first period applies to the first timeseries in each pair, and so for the second
    pub period: (RelativeDuration, RelativeDuration),
    /// number of elements in the second timeseries that correspond to one in the first
    pub ratio: usize,
}

/// A type that can represent either a vector, or a single value
#[derive(Debug, Clone)]
pub enum SingleOrVec<T> {
    /// A single value
    Single(T),
    /// A vector
    Vec(Vec<T>),
}

impl<T> SingleOrVec<T> {
    pub(crate) fn index(&self, index: usize) -> &T {
        match self {
            SingleOrVec::Single(value) => value,
            SingleOrVec::Vec(vec) => vec.index(index),
        }
    }
}

pub(crate) const RADIUS_EARTH: f64 = 6371.0;

pub(crate) fn is_valid(value: f64) -> bool {
    !f64::is_nan(value) && !f64::is_infinite(value)
}

/// convert lat-lon to xyz coordinates
pub(crate) fn convert_coordinates(lat: f64, lon: f64) -> (f64, f64, f64) {
    (
        lat.to_radians().cos() * lon.to_radians().cos() * RADIUS_EARTH,
        lat.to_radians().cos() * lon.to_radians().sin() * RADIUS_EARTH,
        lat.to_radians().sin() * RADIUS_EARTH,
    )
}

/// find the distance in km between two lat-lon points
pub(crate) fn calc_distance(lat1: f64, lon1: f64, lat2: f64, lon2: f64) -> Result<f64, Error> {
    // lons are checked against 360 here, not 180, because some people apparently use
    // conventions of 0-360 and -360-0...
    if lat1.abs() > 90. || lat2.abs() > 90. || lon1.abs() > 360. || lon2.abs() > 360. {
        return Err(Error::InvalidArg(
            "latlon".to_string(),
            "outside valid range".to_string(),
        ));
    }
    if lat1 == lat2 && lon1 == lon2 {
        return Ok(0.);
    }
    let lat1r = lat1.to_radians();
    let lat2r = lat2.to_radians();
    let lon1r = lon1.to_radians();
    let lon2r = lon2.to_radians();

    let ratio = lat1r.cos() * lon1r.cos() * lat2r.cos() * lon2r.cos()
        + lat1r.cos() * lon1r.sin() * lat2r.cos() * lon2r.sin()
        + lat1r.sin() * lat2r.sin();

    // floating point chaos was leading to this leaking outside the 0-1 range that's
    // valid for arccos, hence the enforcement
    let norm_ratio = ratio.clamp(0., 1.);

    Ok(norm_ratio.acos() * RADIUS_EARTH)
}

/// find the distance in km between two xyz points
pub(crate) fn calc_distance_xyz(x0: f64, y0: f64, z0: f64, x1: f64, y1: f64, z1: f64) -> f64 {
    ((x0 - x1) * (x0 - x1) + (y0 - y1) * (y0 - y1) + (z0 - z1) * (z0 - z1)).sqrt()
}
