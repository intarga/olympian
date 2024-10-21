use crate::{DataCache, Flag};

/// Range check with a correction for humidity over 100%.
///
/// Humidity less than 5% or greater than 105% returns Flag::Fail, between 100% and 105% it is,
/// corrected down to 100%.
pub fn range_check_humidity(datum: Option<f32>) -> (Flag, Option<f32>) {
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
pub fn range_check_humidity_cache(cache: &DataCache) -> Vec<(String, Vec<(Flag, Option<f32>)>)> {
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
            windows.map(|datum| range_check_humidity(*datum)).collect(),
        ));
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
                vec![0., 1., 2., 3.],
                vec![0., 1., 2., 3.],
                vec![0., 0., 0., 0.],
                crate::util::Timestamp(0),
                RelativeDuration::minutes(10),
                1,
                1,
                vec![
                    ("blindern1".to_string(), vec![Some(0.), Some(50.), Some(1.)]),
                    ("blindern2".to_string(), vec![Some(0.), Some(3.), None]),
                    (
                        "blindern3".to_string(),
                        vec![Some(0.), Some(103.), Some(1.)]
                    ),
                    ("blindern4".to_string(), vec![Some(1.), None, Some(1.)]),
                ],
            ),),
            vec![
                ("blindern1".to_string(), vec![(Flag::Pass, None)]),
                ("blindern2".to_string(), vec![(Flag::Fail, None)]),
                ("blindern3".to_string(), vec![(Flag::Warn, Some(100.))]),
                ("blindern4".to_string(), vec![(Flag::DataMissing, None)])
            ]
        )
    }
}
