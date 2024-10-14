use crate::{DataCache, Error, Flag};

/// Timeseries QC test that checks for streaks of repeating values.
///
/// If all observations passed in are identical, [`Flag::Fail`] will be returned, if any are
/// missing, [`Flag::DataMissing`], if `data` is empty, [`Flag::Invalid`], else [`Flag::Pass`].
pub fn flatline_check(data: &[Option<f32>]) -> Flag {
    if data.contains(&None) {
        return Flag::DataMissing;
    }
    let data: Vec<f32> = data.iter().map(|opt| opt.unwrap()).collect();

    let base = match data.first() {
        Some(base) => base,
        None => return Flag::Invalid,
    };
    if !data.iter().any(|x| x != base) {
        return Flag::Fail;
    }
    Flag::Pass
}

/// Apply [`flatline_check`] to a whole [`DataCache`]
///
/// If `num_points` observations in a row are identical, the last will be flagged as Flag::Fail, if
/// any of the last `num_points` observations are missing, it will be flagged as Flag::DataMissing,
/// else Flag::Pass.
///
/// As (`num_points` - 1) predecessors to each observation are needed, the [`SeriesCache`] provided must have
/// `num_leading_points` >= `num_points` - 1.
///
/// ## Errors
///
/// - data is invalid
/// - data has `num_leading_points` < `num_points` - 1
pub fn flatline_check_cache(
    cache: &DataCache,
    num_points: u8,
) -> Result<Vec<(String, Vec<Flag>)>, Error> {
    let num_series = cache.data.len();
    let mut result_vec = Vec::with_capacity(cache.data.len());
    let series_len = match cache.data.first() {
        Some(ts) => ts.1.len(),
        // if this is none, the cache is empty, so we can just return an empty result vec
        None => return Ok(result_vec),
    };

    let (leading_trim, lead_overflow) = cache
        .num_leading_points
        .overflowing_sub(num_points.saturating_sub(1));

    if lead_overflow || (leading_trim + num_points) as usize > series_len {
        // TODO: nicer error here?
        return Err(Error::InvalidInputShape("data".to_string()));
    }

    for i in 0..num_series {
        let trimmed = &cache.data[i].1
            [leading_trim as usize..(series_len - cache.num_trailing_points as usize)];

        let windows = trimmed.windows(num_points as usize);

        result_vec.push((cache.data[i].0, windows.map(flatline_check).collect()));
    }

    Ok(result_vec)
}
