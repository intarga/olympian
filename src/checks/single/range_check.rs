use crate::{util::Timeseries, DataCache, Flag};

/// Single check of whether an observation fits within given (inclusive) limits.
///
/// Returns:
/// - [`Flag::DataMissing`] if the observation is missing,
/// - [`Flag::Fail`] if it is outside the upper or lower limits,
/// - [`Flag::Pass`] otherwise.
pub fn range_check(datum: Option<f64>, lower_limit: f64, upper_limit: f64) -> Flag {
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
    lower_limit: f64,
    upper_limit: f64,
) -> Vec<Timeseries<Flag>> {
    let num_series = cache.data.len();
    let mut result_vec = Vec::with_capacity(num_series);
    let series_len = match cache.data.first() {
        Some(ts) => ts.values.len(),
        // if this is none, the cache is empty, so we can just return an empty result vec
        None => return result_vec,
    };

    for i in 0..num_series {
        let trimmed = &cache.data[i].values
            [cache.num_leading_points as usize..(series_len - cache.num_trailing_points as usize)];

        let windows = trimmed.iter();

        result_vec.push(Timeseries {
            tag: cache.data[i].tag.clone(),
            values: windows
                .map(|datum| range_check(*datum, lower_limit, upper_limit))
                .collect(),
        });
    }

    result_vec
}

#[cfg(test)]
mod tests {
    use super::*;
    use chronoutil::RelativeDuration;

    #[test]
    fn test_range_check_cache() {
        assert_eq!(
            range_check_cache(
                &DataCache::new(
                    vec![
                        Timeseries {
                            tag: "blindern1".to_string(),
                            values: vec![Some(0.), Some(0.), None]
                        },
                        Timeseries {
                            tag: "blindern2".to_string(),
                            values: vec![Some(0.), Some(1.), Some(1.)]
                        },
                        Timeseries {
                            tag: "blindern3".to_string(),
                            values: vec![Some(0.), Some(-1.), Some(1.)]
                        },
                        Timeseries {
                            tag: "blindern4".to_string(),
                            values: vec![Some(1.), None, Some(1.)]
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
                0.,
                0.5,
            ),
            vec![
                Timeseries {
                    tag: "blindern1".to_string(),
                    values: vec![Flag::Pass]
                },
                Timeseries {
                    tag: "blindern2".to_string(),
                    values: vec![Flag::Fail]
                },
                Timeseries {
                    tag: "blindern3".to_string(),
                    values: vec![Flag::Fail]
                },
                Timeseries {
                    tag: "blindern4".to_string(),
                    values: vec![Flag::DataMissing]
                }
            ]
        )
    }
}
