use crate::{Flag, SeriesCache};

/// QC test that checks whether each observation matches some special values.
///
/// If the observation is missing, Flag::DataMissing with be returned, else if it is matches any of
/// the special values, Flag::Fail, else Flag::Pass.
pub fn special_values_check(data: &SeriesCache, special_values: &[f32]) -> Vec<Flag> {
    let trimmed = &data.values
        [data.num_leading_points as usize..(data.values.len() - data.num_trailing_points as usize)];

    let windows = trimmed.iter();

    windows
        .map(|data| match data {
            None => Flag::DataMissing,
            Some(data) => {
                if special_values.contains(data) {
                    return Flag::Fail;
                }
                Flag::Pass
            }
        })
        .collect()
}
