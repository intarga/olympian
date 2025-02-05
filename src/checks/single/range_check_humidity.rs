use crate::{util::Timeseries, DataCache, Flag};

/// Range check with a correction for humidity over 100%.
///
/// Returns:
/// - ([`Flag::DataMissing`], None) if the observation is missing,
/// - ([`Flag::Fail`], None) if humidity less than 5% or greater than 105%,
/// - ([`Flag::Warn`], Some(100.)) between 100% and 105%,
/// - ([`Flag::Pass`], None) otherwise.
pub fn range_check_humidity(datum: Option<f64>) -> (Flag, Option<f64>) {
    match datum {
        None => (Flag::DataMissing, None),
        Some(datum) => {
            if !(5. ..105.).contains(&datum) {
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
#[allow(clippy::type_complexity)]
pub fn range_check_humidity_cache(cache: &DataCache) -> Vec<Timeseries<(Flag, Option<f64>)>> {
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
            values: windows.map(|datum| range_check_humidity(*datum)).collect(),
        });
    }

    result_vec
}

#[cfg(test)]
mod tests {
    use super::*;
    use chronoutil::RelativeDuration;

    #[test]
    fn test_range_check_humidity_cache() {
        assert_eq!(
            range_check_humidity_cache(&DataCache::new(
                vec![
                    Timeseries {
                        tag: "blindern1".to_string(),
                        values: vec![Some(0.), Some(50.), Some(1.)]
                    },
                    Timeseries {
                        tag: "blindern2".to_string(),
                        values: vec![Some(0.), Some(3.), None]
                    },
                    Timeseries {
                        tag: "blindern3".to_string(),
                        values: vec![Some(0.), Some(103.), Some(1.)]
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
            ),),
            vec![
                Timeseries {
                    tag: "blindern1".to_string(),
                    values: vec![(Flag::Pass, None)]
                },
                Timeseries {
                    tag: "blindern2".to_string(),
                    values: vec![(Flag::Fail, None)]
                },
                Timeseries {
                    tag: "blindern3".to_string(),
                    values: vec![(Flag::Warn, Some(100.))]
                },
                Timeseries {
                    tag: "blindern4".to_string(),
                    values: vec![(Flag::DataMissing, None)]
                }
            ]
        )
    }
}
