use crate::Flag;

/// Compares a single value to a higher resolution sequence of where the sequence should never be
/// greater than any element of the sequence (including an adjustment)
///
/// If any of the elements break this invariant (i.e they are less than the single value),
/// Both that element and the single value are flagged [`Flag::Fail`]. If any of the elements is
/// missing, that element and the single value are flagged [`Flag::DataMissing`]. Anything that
/// not covered by the above conditions is flagged [`Flag::Pass`]
// NOTE: This could be made more efficient
pub fn single_greater_than_element(
    single: Option<f32>,
    set: &[Option<f32>],
    adjustment: f32,
) -> (Flag, Vec<Flag>) {
    // If the single is missing, or everything in the set is missing, we can't do a check at all
    // so return [`Flag::DataMissing`] for everything
    if single.is_none() || set.iter().all(|elem| elem.is_none()) {
        return (Flag::DataMissing, vec![Flag::DataMissing; set.len()]);
    }

    let set_flags: Vec<Flag> = set
        .iter()
        .map(|elem| match elem {
            Some(value) => {
                if single.unwrap() > value + adjustment {
                    Flag::Fail
                } else {
                    Flag::Pass
                }
            }
            None => Flag::DataMissing,
        })
        .collect();

    let single_flag = if set_flags.contains(&Flag::Fail) {
        Flag::Fail
    } else if set_flags.contains(&Flag::DataMissing) {
        Flag::DataMissing
    } else {
        Flag::Pass
    };

    (single_flag, set_flags)
}
