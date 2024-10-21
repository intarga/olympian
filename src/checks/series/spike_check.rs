use crate::{DataCache, Error, Flag};

/// Number of leading values a [`DataCache`] must contain to QC all its
/// intended values with spike check
pub const SPIKE_LEADING_PER_RUN: u8 = 1;
/// Number of trailing values a [`DataCache`] must contain to QC all its
/// intended values with spike check
pub const SPIKE_TRAILING_PER_RUN: u8 = 1;

/// Timeseries QC test that compares each observation against its immediate predecessor and
/// successor.
///
/// The sum and difference of the differences between the observation and each of its neighbours
/// is computed. the observation will be flagged as follows
/// - If values are missing for the observation or either neighbour: DataMissing.
/// - If the difference is less than 35% of the sum AND the sum is greater than `max`: Fail.
/// - If the difference is less than 35% of the sum AND the sum is greater than `high`: Warn.
/// - Else: Pass
///
/// Takes 3 datapoints, the second is the observation to be QCed, the first and third are needed
/// to QC it.
pub fn spike_check(data: &[Option<f32>; 3], max: f32) -> Flag {
    if data.contains(&None) {
        return Flag::DataMissing;
    }
    let data: Vec<f32> = data.iter().map(|opt| opt.unwrap()).collect();

    if (data[2] < data[1] && data[0] < data[1]) || (data[2] > data[1] && data[0] > data[1]) {
        let diffsum = ((data[2] - data[1]).abs() + (data[1] - data[0]).abs()).abs();
        let diffdiff = ((data[2] - data[1]).abs() - (data[1] - data[0]).abs()).abs();

        if diffdiff < (diffsum * 0.35) && diffsum > max {
            return Flag::Fail;
        }
    }
    Flag::Pass
}

/// Apply [`spike_check`] to a whole [`DataCache`]
///
/// As a predecessor and successor to each observation are needed, the [`SeriesCache`] provided
/// must have `num_leading_points` and `num_trailing_points` >= 1. Constants are provided to aid
/// in enforcing this constraint
///
/// ## Errors
///
/// - data is invalid
/// - data has `num_leading_points` <= 1
/// - data has `num_trailing_points` <= 1
pub fn spike_check_cache(cache: &DataCache, max: f32) -> Result<Vec<(String, Vec<Flag>)>, Error> {
    let num_series = cache.data.len();
    let mut result_vec = Vec::with_capacity(num_series);
    let series_len = match cache.data.first() {
        Some(ts) => ts.1.len(),
        // if this is none, the cache is empty, so we can just return an empty result vec
        None => return Ok(result_vec),
    };

    let (leading_trim, lead_overflow) = cache
        .num_leading_points
        .overflowing_sub(SPIKE_LEADING_PER_RUN);
    let (trailing_trim, trail_overflow) = cache
        .num_trailing_points
        .overflowing_sub(SPIKE_TRAILING_PER_RUN);

    if lead_overflow
        || trail_overflow
        || (leading_trim + trailing_trim + 1 + SPIKE_LEADING_PER_RUN + SPIKE_TRAILING_PER_RUN)
            as usize
            > series_len
    {
        // TODO: nicer error here?
        return Err(Error::InvalidInputShape("data".to_string()));
    }

    for i in 0..num_series {
        let trimmed =
            &cache.data[i].1[leading_trim as usize..(series_len - trailing_trim as usize)];

        let windows = trimmed.windows(3);

        result_vec.push((
            cache.data[i].0.clone(),
            windows
                .map(|data| spike_check(data.try_into().unwrap(), max))
                .collect(),
        ));
    }

    Ok(result_vec)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chronoutil::RelativeDuration;

    #[test]
    fn test_spike_check_cache() {
        assert_eq!(
            spike_check_cache(
                &DataCache::new(
                    vec![0., 1., 2., 3.],
                    vec![0., 1., 2., 3.],
                    vec![0., 0., 0., 0.],
                    crate::util::Timestamp(0),
                    RelativeDuration::minutes(10),
                    1,
                    1,
                    vec![
                        ("blindern1".to_string(), vec![Some(0.), Some(0.), Some(0.)]),
                        ("blindern2".to_string(), vec![Some(0.), Some(1.), Some(1.)]),
                        // This one passes because although the diffsum is enough to be spike,
                        // the diffdiff is big enough to override it
                        ("blindern3".to_string(), vec![Some(0.), Some(1.6), Some(1.)]),
                        (
                            "blindern4".to_string(),
                            vec![Some(0.), Some(-1.1), Some(0.)]
                        ),
                        ("blindern5".to_string(), vec![Some(1.), None, Some(1.)]),
                        ("blindern6".to_string(), vec![None, Some(1.), Some(1.)]),
                    ],
                ),
                1.,
            )
            .unwrap(),
            vec![
                ("blindern1".to_string(), vec![Flag::Pass]),
                ("blindern2".to_string(), vec![Flag::Pass]),
                ("blindern3".to_string(), vec![Flag::Pass]),
                ("blindern4".to_string(), vec![Flag::Fail]),
                ("blindern5".to_string(), vec![Flag::DataMissing]),
                ("blindern6".to_string(), vec![Flag::DataMissing]),
            ]
        )
    }
}
