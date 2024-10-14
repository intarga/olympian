use crate::{DataCache, Flag};

/// Range check with a correction for humidity over 100%.
///
/// Humidity less than 5% or greater than 105% returns Flag::Fail, between 100% and 105% it is,
/// corrected down to 100%.
pub fn range_check_humidity(datum: Option<f32>) -> (Flag, Option<f32>) {
    match datum {
        None => (Flag::DataMissing, None),
        Some(datum) => {
            if datum > 105. || datum < 5. {
                return (Flag::Fail, None);
            } else if datum > 100. {
                // TODO: is Warn the correct flag here?
                return (Flag::Warn, Some(100.));
            }
            (Flag::Pass, None)
        }
    }
}

//TODO: is this the optimal return signature for corrections?
/// Apply [`range_check_humidity`] to a whole [`DataCache`]
pub fn range_check_humidity_cache(cache: &DataCache) -> Vec<(String, Vec<(Flag, Option<f32>)>)> {
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
            windows.map(|datum| range_check_humidity(*datum)).collect(),
        ));
    }

    result_vec
}
