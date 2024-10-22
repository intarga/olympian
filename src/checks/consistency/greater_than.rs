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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_greater_than() {
        assert_eq!(greater_than(Some(1.), Some(1.), 0.2), Flag::Fail);
        assert_eq!(greater_than(Some(1.), Some(1.5), 0.2), Flag::Pass);
        assert_eq!(greater_than(Some(1.), None, 0.2), Flag::DataMissing);
    }
}
