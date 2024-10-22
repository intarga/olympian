use crate::Flag;

/// Consistency check between 2 climate parameters, where both should be within a threshold of
/// each other.
///
/// Useful for comparing observations against model data, or comparing two instruments measuring
/// the same climate parameter at the same site.
///
/// Returns [`Flag::DataMissing`] if either datum is missing,
/// [`Flag::Fail`] if the difference between datum1 and datum2 is greater than threshold,
/// [`Flag::Pass`] otherwise.
pub fn not_equal(datum1: Option<f32>, datum2: Option<f32>, threshold: f32) -> Flag {
    if datum1.is_none() || datum2.is_none() {
        return Flag::DataMissing;
    }

    if (datum1.unwrap() - datum2.unwrap()).abs() > threshold {
        Flag::Fail
    } else {
        Flag::Pass
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_not_equal() {
        assert_eq!(not_equal(Some(1.), Some(1.1), 0.2), Flag::Pass);
        assert_eq!(not_equal(Some(1.), Some(1.2), 0.1), Flag::Fail);
        assert_eq!(not_equal(Some(1.), None, 0.1), Flag::DataMissing);
    }
}
