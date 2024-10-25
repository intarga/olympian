use crate::{util::Timeseries, DataCache, Error, Flag};

/// Timeseries check that looks for streaks of repeating values.
///
/// `threshold` defines the amount that values can differ while still being considered "equal".
/// Even if you are only using this check to find values you expect to exactly the same, it is
/// still useful to give some consideration to threshold here, due to possible noise introduced
/// to floats in transport between systems.
///
/// Returns:
/// - [`Flag::DataMissing`] if any observations are missing,
/// - [`Flag::Invalid`] if `data` is empty,
/// - [`Flag::Fail`] if all observations passed in are identical,
/// - [`Flag::Pass`] otherwise.
pub fn flatline_check(data: &[Option<f32>], threshold: f32) -> Flag {
    if data.contains(&None) {
        return Flag::DataMissing;
    }

    let base = match data.first() {
        Some(base) => base,
        None => return Flag::Invalid,
    };
    if !data
        .iter()
        .any(|x| (x.unwrap() - base.unwrap()).abs() > threshold)
    {
        return Flag::Fail;
    }
    Flag::Pass
}

/// Apply [`flatline_check`] to a whole [`DataCache`]
///
/// `num_points` is the size of rolling windows passed into flatline_check, i.e. how many
/// observations in a row need to be identical to result in a [`Flag::Fail`].
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
    threshold: f32,
) -> Result<Vec<Timeseries<Flag>>, Error> {
    let num_series = cache.data.len();
    let mut result_vec = Vec::with_capacity(cache.data.len());
    let series_len = match cache.data.first() {
        Some(ts) => ts.values.len(),
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
        let trimmed = &cache.data[i].values
            [leading_trim as usize..(series_len - cache.num_trailing_points as usize)];

        let windows = trimmed.windows(num_points as usize);

        result_vec.push(Timeseries {
            tag: cache.data[i].tag.clone(),
            values: windows.map(|x| flatline_check(x, threshold)).collect(),
        });
    }

    Ok(result_vec)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chronoutil::RelativeDuration;

    #[test]
    fn test_flatline_check_cache() {
        assert_eq!(
            flatline_check_cache(
                &DataCache::new(
                    vec![
                        Timeseries {
                            tag: "blindern1".to_string(),
                            values: vec![Some(0.), Some(1.), Some(1.)]
                        },
                        Timeseries {
                            tag: "blindern2".to_string(),
                            values: vec![Some(0.), Some(0.), None]
                        },
                        Timeseries {
                            tag: "blindern3".to_string(),
                            values: vec![Some(1.), None, Some(1.)]
                        },
                        Timeseries {
                            tag: "blindern4".to_string(),
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
                2,
                0.0001
            )
            .unwrap(),
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
                    values: vec![Flag::DataMissing]
                },
                Timeseries {
                    tag: "blindern4".to_string(),
                    values: vec![Flag::DataMissing]
                },
            ]
        )
    }
}
