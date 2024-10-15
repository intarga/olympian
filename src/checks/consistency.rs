mod greater_than;
pub use greater_than::greater_than;

mod aggregate_less_than_set;
pub use aggregate_less_than_set::aggregate_less_than_set;

// TODO: Figure out the ideal container type (Analogous to [`crate::DataCache`]) to pass large
// amounts of data into consistency checks
