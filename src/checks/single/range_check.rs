use crate::{DataCache, Flag};

/// QC test that checks whether each observation fits within given limits.
///
/// If the observation is missing, Flag::DataMissing with be returned, else if it is outside the
/// upper or lower limits, Flag::Fail, else Flag::Pass.
pub fn range_check(datum: Option<f32>, upper_limit: f32, lower_limit: f32) -> Flag {
    match datum {
        None => Flag::DataMissing,
        Some(datum) => {
            if datum > upper_limit || datum < lower_limit {
                return Flag::Fail;
            }
            Flag::Pass
        }
    }
}

/// Apply [`range_check`] to a whole [`DataCache`]
pub fn range_check_cache(
    cache: &DataCache,
    upper_limit: f32,
    lower_limit: f32,
) -> Vec<(String, Vec<Flag>)> {
    let num_series = cache.data.len();
    let mut result_vec = Vec::with_capacity(num_series);
    let series_len = match cache.data.first() {
        Some(ts) => ts.1.len(),
        // if this is none, the cache is empty, so we can just return an empty result vec
        None => return result_vec,
    };

    for i in 0..num_series {
        let trimmed = &cache.data[i].1
            [cache.num_leading_points as usize..(series_len - cache.num_trailing_points as usize)];

        let windows = trimmed.iter();

        result_vec.push((
            cache.data[i].0.clone(),
            windows
                .map(|datum| range_check(*datum, upper_limit, lower_limit))
                .collect(),
        ));
    }

    result_vec
}
