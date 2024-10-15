use crate::Flag;

/// Consistency check between 2 climate parameters, where one (including a correction) should
/// never be greater than the other.
///
/// Returns [`Flag::DataMissing`] if either datum is missing,
/// [`Flag::Fail`] if datum1 + datum1_correction > datum2,
/// [`Flag::Pass`] otherwise.
pub fn greater_than(datum1: Option<f32>, datum2: Option<f32>, datum1_correction: f32) -> Flag {
    if datum1.is_none() || datum2.is_none() {
        return Flag::DataMissing;
    }

    if datum1.unwrap() + datum1_correction > datum2.unwrap() {
        // TODO: confirm these are the right way around
        Flag::Fail
    } else {
        Flag::Pass
    }
}
