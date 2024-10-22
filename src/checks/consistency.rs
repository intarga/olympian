mod greater_than;
pub use greater_than::greater_than;

mod min_greater_than_single;
pub use min_greater_than_single::min_greater_than_single;

mod max_greater_than_single;
pub use max_greater_than_single::max_greater_than_single;

mod min_less_than_single;
pub use min_less_than_single::min_less_than_single;

mod max_less_than_single;
pub use max_less_than_single::max_less_than_single;

mod not_equal;
pub use not_equal::not_equal;

// TODO: Figure out the ideal container type (Analogous to [`crate::DataCache`]) to pass large
// amounts of data into consistency checks
