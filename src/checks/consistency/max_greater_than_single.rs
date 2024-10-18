use crate::Flag;

/// Compares a single value to a higher resolution sequence, where the maximum value in the
/// sequence (including an adjustment) should never be greater than the single value
///
/// If this invariant is broken (i.e the maximum of the sequence, plus the adjustment, is greater
/// than the single value), we return [`Flag::Fail`].
/// Else, if any of the elements is missing, we return [`Flag::DataMissing`], as we cannot be sure
/// a missing data point did not violate the invariant.
/// Else we return [`Flag::Pass`].
pub fn max_greater_than_single(
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

    // find the maximum value in the sequence (None if they are all missing), as well as a bool
    // indicating if any value was missing
    let (max, missing) = sequence
        .iter()
        .fold((None, false), |acc: (Option<f32>, bool), elem| match elem {
            // if the element wasn't missing...
            Some(value) => match acc.0 {
                // set the max if there isn't already a max, or the element is lower
                Some(max) => (Some(max.max(*value)), acc.1),
                None => (Some(*value), acc.1),
            },
            // if the element was missing, leave the max unchanged, but set `missing` to true
            None => (acc.0, true),
        });
    let max = match max {
        Some(value) => value,
        // if min is None at this point, then all the elements of the sequence were missing...
        None => {
            // so we can't perform the check
            return Flag::DataMissing;
        }
    };

    if max + adjustment > single {
        // if this condition evaluates to true, the invariant was invalidated
        Flag::Fail
    } else if missing {
        // if the condition was not met, but there was missing data in the sequence, we cannot say
        // for sure that the invariant wasn't validated
        Flag::DataMissing
    } else {
        Flag::Pass
    }
}
