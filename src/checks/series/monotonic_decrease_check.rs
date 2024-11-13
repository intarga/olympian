use crate::{DataCache, Error, Flag, Timeseries};

/// Timeseries check that detects if a timeseries is monotonically slightly decreasing in a given
/// window. Useful for detecting evaporation/leakage of a bucket measuring precipitation.
///
/// Takes a series of data points representing the window to check for increases, and upper and
/// lower limits that the total difference over the whole series must be within to meet the
/// failure condition
///
/// Since this check is really looking at the intervals between data points, If you are passing in
/// regularly spaced chunks of a time series (e.g. each day's hourly values), you should include
/// an extra point of overlap, so that you check all relevent intervals (e.g. in the aformentioned
/// example you should pass in 25 points, including both the 00:00 and 24:00 points)
///
/// Returns:
/// - [`Flag::DataMissing`] if any of the data points are missing,
/// - [`Flag::Fail`] if there are no increases between consecutive data points in the series
///   AND the total decrease over the series is within the given range
/// - [`Flag::Pass`] otherwise.
pub fn monotonic_decrease_check(data: &[Option<f32>], lower_limit: f32, upper_limit: f32) -> Flag {
    if data.iter().any(Option::is_none) {
        return Flag::DataMissing;
    }

    // take a rolling windows of 2 datapoints over the dataset
    // if the later value is less than the earlier for any of them, there was a decrease
    // apply `!` since we want `true` if there is no decrease
    let no_increase = !data.windows(2).any(|window| window[1] > window[0]);

    let total_diff = data[0].unwrap() - data[24].unwrap();

    if (no_increase) && (lower_limit..=upper_limit).contains(&total_diff) {
        Flag::Fail
    } else {
        Flag::Pass
    }
}

/// Apply [`monotonic_decrease_check`] to a whole [`DataCache`]
///
/// `window_size` here refers to the number of intervals to consider at once, as this is more
/// conceptually straightforward than the number of points (see docs for
/// [`monotonic_decrease_check`]). Each iteration of `monotonic_decrease_check` will actually have
/// `window_size` + 1 points passed to it.
///
/// As `window_size` predecessors to each observation are needed, the [`SeriesCache`]
/// provided must have `num_leading_points` >= `window_size`
///
/// ## Errors
///
/// - data is invalid
/// - data has `num_leading_points` < `window_size`
pub fn monotonic_decrease_check_cache(
    cache: &DataCache,
    lower_limit: f32,
    upper_limit: f32,
    // TODO: should this maybe be a usize? would require changing the type of num_leading_points
    // in DataCache
    window_size: u8,
) -> Result<Vec<Timeseries<Flag>>, Error> {
    let num_series = cache.data.len();
    let mut result_vec = Vec::with_capacity(cache.data.len());
    let series_len = match cache.data.first() {
        Some(ts) => ts.values.len(),
        // if this is none, the cache is empty, so we can just return an empty result vec
        None => return Ok(result_vec),
    };

    let (leading_trim, lead_overflow) = cache.num_leading_points.overflowing_sub(window_size);

    if lead_overflow || (leading_trim + window_size + 1) as usize > series_len {
        // TODO: nicer error here?
        return Err(Error::InvalidInputShape("data".to_string()));
    }

    for i in 0..num_series {
        let trimmed = &cache.data[i].values
            [leading_trim as usize..(series_len - cache.num_trailing_points as usize)];

        // plus 1 to account for the extra data point needed to get an interval corresponding to
        // the first interval of the window
        let windows = trimmed.windows(window_size as usize + 1);

        result_vec.push(Timeseries {
            tag: cache.data[i].tag.clone(),
            values: windows
                .map(|x| monotonic_decrease_check(x, lower_limit, upper_limit))
                .collect(),
        });
    }

    Ok(result_vec)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chronoutil::RelativeDuration;
    use std::iter::repeat;

    #[test]
    fn test_monotonic_decrease_check() {
        let decreasing_sequence: Vec<Option<f32>> = repeat(200.)
            .take(25)
            .enumerate()
            .map(|(i, val)| Some(val - (i as f32 * 0.1)))
            .collect();
        println!("decreasing_sequence: {:?}", decreasing_sequence);
        assert_eq!(
            monotonic_decrease_check(&decreasing_sequence.clone(), 0.7, 100.),
            Flag::Fail
        );

        let non_decreasing_sequence = {
            let mut s = decreasing_sequence;
            s[10] = Some(s[10].unwrap() + 0.2);
            s
        };
        assert_eq!(
            monotonic_decrease_check(&non_decreasing_sequence, 0.7, 100.),
            Flag::Pass
        );
    }

    #[test]
    fn test_monotonic_decrease_check_cache() {
        let decreasing_sequence: Vec<Option<f32>> = repeat(200.)
            .take(27)
            .enumerate()
            .map(|(i, val)| Some(val - (i as f32 * 0.1)))
            .collect();
        let increase_in_middle = {
            let mut s = decreasing_sequence.clone();
            s[10] = Some(s[10].unwrap() + 0.2);
            s
        };
        let increase_at_start = {
            let mut s = decreasing_sequence.clone();
            s[1] = Some(s[1].unwrap() + 0.2);
            s
        };
        let increase_at_end = {
            let mut s = decreasing_sequence.clone();
            s[26] = Some(s[26].unwrap() + 0.2);
            s
        };
        let missing_at_end = {
            let mut s = decreasing_sequence.clone();
            s[26] = None;
            s
        };
        let big_decrease_at_end = {
            let mut s = decreasing_sequence.clone();
            s[26] = Some(s[26].unwrap() - 100.);
            s
        };
        assert_eq!(
            monotonic_decrease_check_cache(
                &DataCache::new(
                    vec![
                        Timeseries {
                            tag: "blindern1".to_string(),
                            values: decreasing_sequence
                        },
                        Timeseries {
                            tag: "blindern2".to_string(),
                            values: increase_in_middle
                        },
                        Timeseries {
                            tag: "blindern3".to_string(),
                            values: increase_at_start
                        },
                        Timeseries {
                            tag: "blindern4".to_string(),
                            values: increase_at_end
                        },
                        Timeseries {
                            tag: "blindern5".to_string(),
                            values: missing_at_end
                        },
                        Timeseries {
                            tag: "blindern6".to_string(),
                            values: big_decrease_at_end
                        },
                    ],
                    vec![0., 1., 2., 3., 4., 5.],
                    vec![0., 1., 2., 3., 4., 5.],
                    vec![0., 0., 0., 0., 0., 0.],
                    crate::util::Timestamp(0),
                    RelativeDuration::minutes(10),
                    24,
                    0,
                ),
                0.7,
                100.,
                24
            )
            .unwrap(),
            vec![
                Timeseries {
                    tag: "blindern1".to_string(),
                    values: vec![Flag::Fail, Flag::Fail, Flag::Fail,]
                },
                Timeseries {
                    tag: "blindern2".to_string(),
                    values: vec![Flag::Pass, Flag::Pass, Flag::Pass,]
                },
                Timeseries {
                    tag: "blindern3".to_string(),
                    values: vec![Flag::Pass, Flag::Fail, Flag::Fail,]
                },
                Timeseries {
                    tag: "blindern4".to_string(),
                    values: vec![Flag::Fail, Flag::Fail, Flag::Pass,]
                },
                Timeseries {
                    tag: "blindern5".to_string(),
                    values: vec![Flag::Fail, Flag::Fail, Flag::DataMissing,]
                },
                Timeseries {
                    tag: "blindern6".to_string(),
                    values: vec![Flag::Fail, Flag::Fail, Flag::Pass,]
                },
            ]
        )
    }
}
