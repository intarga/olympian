use crate::{DataCache, Error, Flag};

/// Number of leading values a [`DataCache`] must contain to QC all its
/// intended values with step check
pub const STEP_LEADING_PER_RUN: u8 = 1;

/// Timeseries QC test that compares each observation against its immediate predecessor.
///
/// If the absolute value of the difference between the observed value and it's predecessor is
/// greater than max, Flag::Fail will be returned for that observation, if greater than high,
/// Flag::Warn, if either value if missing, Flag::DataMissing, else Flag::Pass.
///
/// Takes 2 datapoints, the second is the observation to be QCed, the first is needed to QC it.
pub fn step_check(data: &[Option<f32>; 2], max: f32) -> Flag {
    if data.contains(&None) {
        return Flag::DataMissing;
    }
    let data: Vec<f32> = data.iter().map(|opt| opt.unwrap()).collect();

    if (data[0] - data[1]).abs() > max {
        return Flag::Fail;
    }
    Flag::Pass
}

/// Apply [`step_check`] to a whole [`DataCache`]
///
/// As a predecessor to each observation is needed, the [`SeriesCache`] provided must have
/// `num_leading_points` >= 1.
///
/// As a predecessor and successor to each observation are needed, the [`SeriesCache`] provided
/// must have `num_leading_points` and `num_trailing_points` >= 1. A constant is provided to aid
/// in enforcing this constraint
///
/// ## Errors
///
/// - data is invalid
/// - data has `num_leading_points` <= 1
pub fn step_check_cache(cache: &DataCache, max: f32) -> Result<Vec<(String, Vec<Flag>)>, Error> {
    let num_series = cache.data.len();
    let mut result_vec = Vec::with_capacity(num_series);
    let series_len = match cache.data.first() {
        Some(ts) => ts.1.len(),
        // if this is none, the cache is empty, so we can just return an empty result vec
        None => return Ok(result_vec),
    };

    let (leading_trim, lead_overflow) = cache
        .num_leading_points
        .overflowing_sub(STEP_LEADING_PER_RUN);

    if lead_overflow || (leading_trim + 2) as usize > series_len {
        // TODO: nicer error here?
        return Err(Error::InvalidInputShape("data".to_string()));
    }

    for i in 0..num_series {
        let trimmed = &cache.data[i].1
            [leading_trim as usize..(series_len - cache.num_trailing_points as usize)];

        let windows = trimmed.windows(1 + STEP_LEADING_PER_RUN as usize);

        result_vec.push((
            cache.data[i].0.clone(),
            windows
                .map(|data| step_check(data.try_into().unwrap(), max))
                .collect(),
        ))
    }

    Ok(result_vec)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chronoutil::RelativeDuration;

    #[test]
    fn test_step_check_cache() {
        assert_eq!(
            step_check_cache(
                &DataCache::new(
                    vec![0., 1., 2., 3.],
                    vec![0., 1., 2., 3.],
                    vec![0., 0., 0., 0.],
                    crate::util::Timestamp(0),
                    RelativeDuration::minutes(10),
                    1,
                    1,
                    vec![
                        ("blindern1".to_string(), vec![Some(0.), Some(0.), None]),
                        ("blindern2".to_string(), vec![Some(0.), Some(1.), Some(1.)]),
                        (
                            "blindern3".to_string(),
                            vec![Some(0.), Some(-1.1), Some(1.)]
                        ),
                        ("blindern4".to_string(), vec![Some(1.), None, Some(1.)]),
                        ("blindern5".to_string(), vec![None, Some(1.), Some(1.)]),
                    ],
                ),
                1.,
            )
            .unwrap(),
            vec![
                ("blindern1".to_string(), vec![Flag::Pass]),
                ("blindern2".to_string(), vec![Flag::Pass]),
                ("blindern3".to_string(), vec![Flag::Fail]),
                ("blindern4".to_string(), vec![Flag::DataMissing]),
                ("blindern5".to_string(), vec![Flag::DataMissing]),
            ]
        )
    }
}
