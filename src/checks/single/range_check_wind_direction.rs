use crate::{DataCache, Flag};

/// Range check with a correction for wind direction outside 0-360.
///
/// If the direction is -20-0, or 360-380, 360 will be added or subtracted to get it back into
/// the correct range.
pub fn range_check_wind_direction(datum: Option<f32>) -> (Flag, Option<f32>) {
    // TODO: get to the bottom of weird -3.0 handling: kvalobs code looks for a value -3.0, and
    // avoids flagging that if X_5 (lowest?) is also -3.0. From comments in the code, it looks like
    // this has to do with a special param_id?

    // TODO: figure out what the Y param in kvalobs code is. Windspeed? Why check it's not zero for
    // the correction runs, but not the first check? if it is windspeed should they be qced
    // together as one param?

    match datum {
        None => (Flag::DataMissing, None),
        Some(datum) => {
            if !(-20. ..=380.).contains(&datum) {
                return (Flag::Fail, None);
            } else if datum < 0. {
                // TODO: is Warn the correct flag here?
                return (Flag::Warn, Some(datum + 360.));
            } else if datum >= 360. {
                return (Flag::Warn, Some(datum - 360.));
            }
            (Flag::Pass, None)
        }
    }
}

//TODO: is this the optimal return signature for corrections?
/// Apply [`range_check_wind_direction`] to a whole [`DataCache`]
#[allow(clippy::type_complexity)]
pub fn range_check_wind_direction_cache(
    cache: &DataCache,
) -> Vec<(String, Vec<(Flag, Option<f32>)>)> {
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
                .map(|datum| range_check_wind_direction(*datum))
                .collect(),
        ));
    }

    result_vec
}

#[cfg(test)]
mod tests {
    use super::*;
    use chronoutil::RelativeDuration;

    #[test]
    fn test_range_check_wind_direction_cache() {
        assert_eq!(
            range_check_wind_direction_cache(&DataCache::new(
                vec![0., 1., 2., 3.],
                vec![0., 1., 2., 3.],
                vec![0., 0., 0., 0.],
                crate::util::Timestamp(0),
                RelativeDuration::minutes(10),
                1,
                1,
                vec![
                    (
                        "blindern1".to_string(),
                        vec![Some(0.), Some(120.), Some(390.)]
                    ),
                    ("blindern2".to_string(), vec![Some(0.), Some(390.), None]),
                    ("blindern3".to_string(), vec![Some(0.), Some(-5.), Some(1.)]),
                    ("blindern4".to_string(), vec![Some(390.), None, Some(1.)]),
                ],
            ),),
            vec![
                ("blindern1".to_string(), vec![(Flag::Pass, None)]),
                ("blindern2".to_string(), vec![(Flag::Fail, None)]),
                ("blindern3".to_string(), vec![(Flag::Warn, Some(355.))]),
                ("blindern4".to_string(), vec![(Flag::DataMissing, None)])
            ]
        )
    }
}
