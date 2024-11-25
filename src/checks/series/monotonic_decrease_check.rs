use crate::{DataCache, Error, Flag, Timeseries};

/// Timeseries check that detects if a timeseries is monotonically slightly decreasing in a given
/// window. Useful for detecting evaporation/leakage of a bucket measuring precipitation.
///
/// The total difference over the whole series provided in `data` must be within the provided upper
/// and lower limits to meet the failure condition. In the case of detecting evaporation/leakage,
/// the lower limit is used to discount false positives due to noise, and the upper limit is used
/// to discount manual emptying of the bucket.
///
/// Since this check is really looking at the intervals between data points, If you are passing in
/// regularly spaced chunks of a time series (e.g. each day's hourly values), you should include
/// an extra point of overlap, so that you check all relevent intervals (e.g. in the aformentioned
/// example you should pass in 25 points, including both the 00:00 and 24:00 points)
///
/// Returns:
/// - [`Flag::DataMissing`] if any of the data points are missing, or the `data` slice is empty
/// - [`Flag::Fail`] if there are no increases between consecutive data points in the series
///   AND the total decrease over the series is within the given range
/// - [`Flag::Pass`] otherwise.
pub fn monotonic_decrease_check(data: &[Option<f32>], lower_limit: f32, upper_limit: f32) -> Flag {
    if data.is_empty() || data.iter().any(Option::is_none) {
        return Flag::DataMissing;
    }

    // take a rolling windows of 2 datapoints over the dataset
    // if the later value is less than the earlier for any of them, there was a decrease
    // apply `!` since we want `true` if there is no decrease
    let no_increase = !data.windows(2).any(|window| window[1] > window[0]);

    let total_diff = data.first().unwrap().unwrap() - data.last().unwrap().unwrap();

    if (no_increase) && (lower_limit..=upper_limit).contains(&total_diff) {
        Flag::Fail
    } else {
        Flag::Pass
    }
}

// TODO: It might be possible to write a more efficent version of this function that doesn't call
// down to `monotonic_decrease_check`
/// Apply [`monotonic_decrease_check`] to a whole [`DataCache`]
///
/// `window_size` here refers to the number of intervals to consider at once (i.e. the number of
/// points in a row that must be decreasing to be flagged as [`Flag::Fail`]), as this is more
/// conceptually straightforward than the number of points (see docs for
/// [`monotonic_decrease_check`]). Each iteration of `monotonic_decrease_check` will actually have
/// `window_size` + 1 points passed to it. That is to say, if you're QCing hourly data, and want to
/// check for decreases over 24 hour periods, you should specify a `window_size` of 24, but
/// internally when this function calls down to [`monotonic_decrease_check`] it will actually pass
/// it 25 values.
///
/// Since [`monotonic_decrease_check`] returns a flag relevant to the whole window passed in, but we
/// want flags for individual data points, we consider flags [`monotonic_decrease check`] returns
/// for every window that contains a given data point, and coalesce them according to the following
/// precedence: [`Flag::Fail`] > [`Flag::DataMissing`] > [`Flag::Pass`].
///
/// As `window_size` predecessors and successors to each observation are needed to have all the
/// windows relevent to the observation, the [`SeriesCache`] provided must have
/// `num_leading_points` and `num_trailing_points` >= `window_size`
///
/// ## Errors
///
/// - data is invalid
/// - data has `num_leading_points` or `num_trailing_points` < `window_size`
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
    let qced_series_len =
        series_len - (cache.num_leading_points as usize + cache.num_trailing_points as usize);

    let (leading_trim, lead_overflow) = cache.num_leading_points.overflowing_sub(window_size);
    let (trailing_trim, trail_overflow) = cache.num_trailing_points.overflowing_sub(window_size);

    if lead_overflow
        || trail_overflow
        || (leading_trim + trailing_trim + window_size + 1) as usize > series_len
    {
        // TODO: nicer error here?
        return Err(Error::InvalidInputShape("data".to_string()));
    }

    for i in 0..num_series {
        let trimmed =
            &cache.data[i].values[leading_trim as usize..(series_len - trailing_trim as usize)];

        // plus 1 to account for the extra data point needed to get an interval corresponding to
        // the first interval of the window
        let windows = trimmed.windows(window_size as usize + 1);

        let window_flags: Vec<Flag> = windows
            .map(|x| monotonic_decrease_check(x, lower_limit, upper_limit))
            .collect();

        // each point that actually needs to be QCed has `window_size + 1` windows that contain it,
        // and so `window_size + 1` flags in `window_flags` associated with it which need to be
        // coalesced, with `Fail` taking highest priority, then `DataMissing`.
        let mut point_flags = Vec::with_capacity(qced_series_len);
        for i in 0..qced_series_len {
            let mut point_flag = Flag::Pass;
            for j in 0..(window_size as usize + 1) {
                if window_flags[i + j] == Flag::Fail {
                    point_flag = Flag::Fail;
                    break;
                } else if window_flags[i + j] == Flag::DataMissing {
                    point_flag = Flag::DataMissing;
                }
            }
            point_flags.push(point_flag);
        }

        result_vec.push(Timeseries {
            tag: cache.data[i].tag.clone(),
            values: point_flags,
        })
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
        assert_eq!(
            monotonic_decrease_check_cache(
                &DataCache::new(
                    vec![
                        Timeseries {
                            tag: "blindern1".to_string(),
                            // Should be [Fail, Fail]. All windows are slightly decreasing
                            values: vec![
                                Some(100.),
                                Some(99.),
                                Some(98.),
                                Some(97.),
                                Some(96.),
                                Some(95.),
                            ]
                        },
                        Timeseries {
                            tag: "blindern2".to_string(),
                            // Should be [Fail, Pass]. There's a decreasing window at 0..2, but
                            // that window is only relevant for the first value
                            values: vec![
                                Some(100.),
                                Some(99.),
                                Some(98.),
                                Some(100.),
                                Some(100.),
                                Some(100.),
                            ]
                        },
                        Timeseries {
                            tag: "blindern3".to_string(),
                            // Should be [Pass, Fail]. There's a decreasing window at 3..5, but
                            // that window is only relevant for the second value
                            values: vec![
                                Some(100.),
                                Some(100.),
                                Some(100.),
                                Some(100.),
                                Some(100.),
                                Some(95.),
                            ]
                        },
                        Timeseries {
                            tag: "blindern4".to_string(),
                            // Should be [Fail, Fail]. There's an non-decreasing window at 3..5,
                            // but the second value still has 2 decreasing windows relevant to it
                            values: vec![
                                Some(100.),
                                Some(99.),
                                Some(98.),
                                Some(97.),
                                Some(96.),
                                Some(97.),
                            ]
                        },
                        Timeseries {
                            tag: "blindern5".to_string(),
                            // Should be [Pass, Pass]. Always increasing, so no decreasing sequence
                            values: vec![
                                Some(100.),
                                Some(101.),
                                Some(102.),
                                Some(103.),
                                Some(104.),
                                Some(105.),
                            ]
                        },
                        Timeseries {
                            tag: "blindern6".to_string(),
                            // Should be [Pass, Pass]. The sequence is decreasing, but by less than
                            // lower_limit
                            values: vec![
                                Some(100.),
                                Some(99.9),
                                Some(99.8),
                                Some(99.7),
                                Some(99.6),
                                Some(99.5),
                            ]
                        },
                        Timeseries {
                            // Should be [Pass, DataMissing]. This along with the following case,
                            // checks that DataMissing in one window takes precedence over Passes
                            // in the others, but not over Fails
                            tag: "blindern7".to_string(),
                            values: vec![
                                Some(100.),
                                Some(101.),
                                Some(102.),
                                Some(103.),
                                Some(104.),
                                None,
                            ]
                        },
                        Timeseries {
                            // Should be [Fail, Fail]. See the above case for an explanation
                            tag: "blindern8".to_string(),
                            values: vec![
                                Some(100.),
                                Some(99.),
                                Some(98.),
                                Some(97.),
                                Some(96.),
                                None,
                            ]
                        },
                        Timeseries {
                            tag: "blindern9".to_string(),
                            // Should be [Pass, Pass]. Represents the case of the bucket being
                            // emptied. There are decreasing sequences, but they're by more than
                            // upper_limit
                            values: vec![
                                Some(100.),
                                Some(100.),
                                Some(100.),
                                Some(0.),
                                Some(0.),
                                Some(0.),
                            ]
                        },
                    ],
                    vec![0., 1., 2., 3., 4., 5., 6., 7., 8.,],
                    vec![0., 1., 2., 3., 4., 5., 6., 7., 8.,],
                    vec![0., 0., 0., 0., 0., 0., 0., 0., 0.,],
                    crate::util::Timestamp(0),
                    RelativeDuration::minutes(10),
                    2,
                    2,
                ),
                1.,
                50.,
                2
            )
            .unwrap(),
            vec![
                Timeseries {
                    tag: "blindern1".to_string(),
                    values: vec![Flag::Fail, Flag::Fail]
                },
                Timeseries {
                    tag: "blindern2".to_string(),
                    values: vec![Flag::Fail, Flag::Pass]
                },
                Timeseries {
                    tag: "blindern3".to_string(),
                    values: vec![Flag::Pass, Flag::Fail]
                },
                Timeseries {
                    tag: "blindern4".to_string(),
                    values: vec![Flag::Fail, Flag::Fail]
                },
                Timeseries {
                    tag: "blindern5".to_string(),
                    values: vec![Flag::Pass, Flag::Pass]
                },
                Timeseries {
                    tag: "blindern6".to_string(),
                    values: vec![Flag::Pass, Flag::Pass]
                },
                Timeseries {
                    tag: "blindern7".to_string(),
                    values: vec![Flag::Pass, Flag::DataMissing]
                },
                Timeseries {
                    tag: "blindern8".to_string(),
                    values: vec![Flag::Fail, Flag::Fail]
                },
                Timeseries {
                    tag: "blindern9".to_string(),
                    values: vec![Flag::Pass, Flag::Pass]
                },
            ]
        )
    }
}
