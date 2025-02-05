use crate::{util::Timeseries, DataCache, Error, Flag};

/// Number of leading values a [`DataCache`] must contain to QC all its
/// intended values with spike check
pub const SPIKE_LEADING_PER_RUN: u8 = 1;
/// Number of trailing values a [`DataCache`] must contain to QC all its
/// intended values with spike check
pub const SPIKE_TRAILING_PER_RUN: u8 = 1;

/// Timeseries check that compares each observation against its immediate predecessor and
/// successor.
///
/// Takes 3 datapoints, the second is the observation to be QCed, the first and third are needed
/// to QC it.
///
/// The sum and difference of the differences between the observation and each of its neighbours
/// is computed.
///
/// Returns:
/// - [`Flag::DataMissing`] if values are missing for the observation or either neighbour,
/// - [`Flag::Fail`] if the difference (explained above) is less than 35% of the sum AND the sum
///   (explained above) is greater than `max`,
/// - [`Flag::Pass`] otherwise.
pub fn spike_check(data: &[Option<f64>; 3], max: f64) -> Flag {
    if data.contains(&None) {
        return Flag::DataMissing;
    }
    let (left, obs, right) = (data[0].unwrap(), data[1].unwrap(), data[2].unwrap());

    if (right < obs && left < obs) || (right > obs && left > obs) {
        let diffsum = ((right - obs).abs() + (obs - left).abs()).abs();
        let diffdiff = ((right - obs).abs() - (obs - left).abs()).abs();

        if diffdiff < (diffsum * 0.35) && diffsum > max {
            return Flag::Fail;
        }
    }
    Flag::Pass
}

/// Apply [`spike_check`] to a whole [`DataCache`]
///
/// As a predecessor and successor to each observation are needed, the [`DataCache`] provided
/// must have `num_leading_points` and `num_trailing_points` >= 1. Constants are provided to aid
/// in enforcing this constraint
///
/// ## Errors
///
/// - data is invalid
/// - data has `num_leading_points` <= 1
/// - data has `num_trailing_points` <= 1
pub fn spike_check_cache(cache: &DataCache, max: f64) -> Result<Vec<Timeseries<Flag>>, Error> {
    let num_series = cache.data.len();
    let mut result_vec = Vec::with_capacity(num_series);
    let series_len = match cache.data.first() {
        Some(ts) => ts.values.len(),
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
            &cache.data[i].values[leading_trim as usize..(series_len - trailing_trim as usize)];

        let windows = trimmed.windows(3);

        result_vec.push(Timeseries {
            tag: cache.data[i].tag.clone(),
            values: windows
                .map(|data| spike_check(data.try_into().unwrap(), max))
                .collect(),
        });
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
                    vec![
                        Timeseries {
                            tag: "blindern1".to_string(),
                            values: vec![Some(0.), Some(0.), Some(0.)]
                        },
                        Timeseries {
                            tag: "blindern2".to_string(),
                            values: vec![Some(0.), Some(1.), Some(1.)]
                        },
                        // This one passes because although the diffsum is enough to be spike,
                        // the diffdiff is big enough to override it
                        Timeseries {
                            tag: "blindern3".to_string(),
                            values: vec![Some(0.), Some(1.6), Some(1.)]
                        },
                        Timeseries {
                            tag: "blindern4".to_string(),
                            values: vec![Some(0.), Some(-1.1), Some(0.)]
                        },
                        Timeseries {
                            tag: "blindern5".to_string(),
                            values: vec![Some(1.), None, Some(1.)]
                        },
                        Timeseries {
                            tag: "blindern6".to_string(),
                            values: vec![None, Some(1.), Some(1.)]
                        },
                    ],
                    vec![0., 1., 2., 3.],
                    vec![0., 1., 2., 3.],
                    vec![0., 0., 0., 0.],
                    crate::util::Timestamp(0),
                    RelativeDuration::minutes(10),
                    1,
                    1,
                ),
                1.,
            )
            .unwrap(),
            vec![
                Timeseries {
                    tag: "blindern1".to_string(),
                    values: vec![Flag::Pass]
                },
                Timeseries {
                    tag: "blindern2".to_string(),
                    values: vec![Flag::Pass]
                },
                Timeseries {
                    tag: "blindern3".to_string(),
                    values: vec![Flag::Pass]
                },
                Timeseries {
                    tag: "blindern4".to_string(),
                    values: vec![Flag::Fail]
                },
                Timeseries {
                    tag: "blindern5".to_string(),
                    values: vec![Flag::DataMissing]
                },
                Timeseries {
                    tag: "blindern6".to_string(),
                    values: vec![Flag::DataMissing]
                },
            ]
        )
    }
}
