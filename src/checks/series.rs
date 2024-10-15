mod flatline_check;
pub use flatline_check::{flatline_check, flatline_check_cache};

mod spike_check;
pub use spike_check::{
    spike_check, spike_check_cache, SPIKE_LEADING_PER_RUN, SPIKE_TRAILING_PER_RUN,
};

mod step_check;
pub use step_check::{step_check, step_check_cache, STEP_LEADING_PER_RUN};

mod monotonic_increase_check;
pub use monotonic_increase_check::monotonic_increase_check;
