use crate::{Flag, SeriesCache};

/// QC test that checks whether each observation fits within given limits.
///
/// If the observation is missing, Flag::DataMissing with be returned, else if it is outside the
/// upper or lower limits, Flag::Fail, else Flag::Pass.
pub fn range_check(data: &SeriesCache, upper_limit: f32, lower_limit: f32) -> Vec<Flag> {
    let trimmed = &data.values
        [data.num_leading_points as usize..(data.values.len() - data.num_trailing_points as usize)];

    let windows = trimmed.iter();

    windows
        .map(|data| match data {
            None => Flag::DataMissing,
            Some(data) => {
                if *data > upper_limit || *data < lower_limit {
                    return Flag::Fail;
                }
                Flag::Pass
            }
        })
        .collect()
}
