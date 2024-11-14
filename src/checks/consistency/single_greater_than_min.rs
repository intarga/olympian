use crate::Flag;

/// Compares a single value to a higher resolution sequence, where the single value should never
/// be greater than the minimum value in the sequence (including an adjustment)
///
/// Returns:
/// - [`Flag::DataMissing`] if the single value is missing,
/// - [`Flag::Fail`] if this invariant is broken (i.e the single value is greater than the minimum
///   of the sequence plus the adjustment),
/// - [`Flag::DataMissing`] if any of the elements is missing, as we cannot be sure a missing data
///   point did not violate the invariant.
/// - [`Flag::Pass`] otherwise.
pub fn single_greater_than_min(
    single: Option<f32>,
    sequence: &[Option<f32>],
    adjustment: f32,
) -> Flag {
    let single = match single {
        Some(value) => value,
        None => {
            // If the single is missing, we can't do a check at all
            return Flag::DataMissing;
        }
    };

    // find the minimum value in the sequence (None if they are all missing), as well as a bool
    // indicating if any value was missing
    let (min, missing) = sequence
        .iter()
        .fold((None, false), |acc: (Option<f32>, bool), elem| match elem {
            // if the element wasn't missing...
            Some(value) => match acc.0 {
                // set the min if there isn't already a min, or the element is lower
                Some(min) => (Some(min.min(*value)), acc.1),
                None => (Some(*value), acc.1),
            },
            // if the element was missing, leave the min unchanged, but set `missing` to true
            None => (acc.0, true),
        });
    let min = match min {
        Some(value) => value,
        // if min is None at this point, then all the elements of the sequence were missing...
        None => {
            // so we can't perform the check
            return Flag::DataMissing;
        }
    };

    if single > min + adjustment {
        // if this condition evaluates to true, the invariant was invalidated
        Flag::Fail
    } else if missing {
        // if the condition was not met, but there was missing data in the sequence, we cannot say
        // for sure that the invariant wasn't invalidated
        Flag::DataMissing
    } else {
        Flag::Pass
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_single_greater_than_min() {
        assert_eq!(
            single_greater_than_min(Some(1.), &[Some(1.), Some(2.), Some(2.)], 0.2),
            Flag::Pass
        );
        assert_eq!(
            single_greater_than_min(Some(1.), &[Some(1.), Some(2.), Some(2.)], -0.2),
            Flag::Fail
        );
        assert_eq!(
            single_greater_than_min(Some(1.), &[Some(1.), None, Some(2.)], -0.2),
            Flag::Fail
        );
        assert_eq!(
            single_greater_than_min(Some(1.), &[Some(1.), None, Some(2.)], 0.2),
            Flag::DataMissing
        );
    }
}
