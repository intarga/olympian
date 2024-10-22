use crate::{DataCache, Flag};

/// QC test that checks whether each observation matches some special values.
///
/// If the observation is missing, Flag::DataMissing with be returned, else if it is matches any of
/// the special values, Flag::Fail, else Flag::Pass.
pub fn special_values_check(datum: Option<f32>, special_values: &[f32]) -> Flag {
    match datum {
        None => Flag::DataMissing,
        Some(datum) => {
            if special_values.contains(&datum) {
                return Flag::Fail;
            }
            Flag::Pass
        }
    }
}

/// Apply [`special_values_check`] to a whole [`DataCache`]
pub fn special_values_check_cache(
    cache: &DataCache,
    special_values: &[f32],
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
                .map(|datum| special_values_check(*datum, special_values))
                .collect(),
        ))
    }

    result_vec
}

#[cfg(test)]
mod tests {
    use super::*;
    use chronoutil::RelativeDuration;

    #[test]
    fn test_special_values_check_cache() {
        assert_eq!(
            special_values_check_cache(
                &DataCache::new(
                    vec![0., 1., 2.],
                    vec![0., 1., 2.],
                    vec![0., 0., 0.],
                    crate::util::Timestamp(0),
                    RelativeDuration::minutes(10),
                    1,
                    1,
                    vec![
                        ("blindern1".to_string(), vec![Some(0.), Some(0.), None]),
                        ("blindern2".to_string(), vec![Some(0.), Some(1.), Some(1.)]),
                        ("blindern3".to_string(), vec![Some(1.), None, Some(1.)]),
                    ],
                ),
                &vec![1., 2.]
            ),
            vec![
                ("blindern1".to_string(), vec![Flag::Pass]),
                ("blindern2".to_string(), vec![Flag::Fail]),
                ("blindern3".to_string(), vec![Flag::DataMissing])
            ]
        )
    }
}
