/// Checks of consistency between two timeseries.
pub mod consistency;
/// Checks that operate on single timeseries.
pub mod series;
/// Checks that operate on single pieces of data.
pub mod single;
/// Checks that operate on spatially distributed data.
///
/// (Data from different stations at the same timestamp)
pub mod spatial;
