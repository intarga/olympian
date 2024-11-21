use crate::{ConsistencyCache, Error, Flag, Timeseries, TimeseriesPair};

/// Consistency check between 2 climate parameters, where both should be within a threshold of
/// each other.
///
/// Useful for comparing observations against model data, or comparing two instruments measuring
/// the same climate parameter at the same site.
///
/// Returns:
/// - [`Flag::DataMissing`] if either datum is missing,
/// - [`Flag::Fail`] if the difference between datum1 and datum2 is greater than threshold,
/// - [`Flag::Pass`] otherwise.
pub fn not_equal(datum1: Option<f32>, datum2: Option<f32>, threshold: f32) -> Flag {
    if datum1.is_none() || datum2.is_none() {
        return Flag::DataMissing;
    }

    if (datum1.unwrap() - datum2.unwrap()).abs() > threshold {
        Flag::Fail
    } else {
        Flag::Pass
    }
}

/// Apply [`not_equal`] to a whole [`ConsistencyCache`]
///
/// `cache.ratio` must be 1, as that is the only alignment that makes sense for [`not_equal`]
///
/// ## Errors
///
/// - `cache.ratio` is not 1
pub fn not_equal_cache(
    cache: &ConsistencyCache,
    threshold: f32,
) -> Result<Vec<TimeseriesPair<Flag>>, Error> {
    if cache.ratio != 1 {
        return Err(Error::InvalidInputShape(String::from(
            "arg cache in greater_than_cache must have cache.ratio == 1",
        )));
    }

    let num_series = cache.data.len();
    let mut result_vec = Vec::with_capacity(num_series);

    for i in 0..num_series {
        let (series1, series2) = &cache.data[i];

        let flags: Vec<Flag> = series1
            .values
            .iter()
            .zip(series2.values.iter())
            .map(|(datum1, datum2)| not_equal(*datum1, *datum2, threshold))
            .collect();

        result_vec.push((
            Timeseries {
                tag: series1.tag.clone(),
                values: flags.clone(),
            },
            Timeseries {
                tag: series2.tag.clone(),
                values: flags,
            },
        ))
    }

    Ok(result_vec)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chronoutil::RelativeDuration;

    #[test]
    fn test_not_equal() {
        assert_eq!(not_equal(Some(1.), Some(1.1), 0.2), Flag::Pass);
        assert_eq!(not_equal(Some(1.), Some(1.2), 0.1), Flag::Fail);
        assert_eq!(not_equal(Some(1.), None, 0.1), Flag::DataMissing);
    }

    #[test]
    fn test_not_equal_cache() {
        assert_eq!(
            not_equal_cache(
                &ConsistencyCache {
                    data: vec![(
                        Timeseries {
                            tag: String::from("blindern1"),
                            values: vec![Some(1.), Some(1.), Some(1.)]
                        },
                        Timeseries {
                            tag: String::from("blindern2"),
                            values: vec![Some(1.1), Some(1.3), None]
                        }
                    )],
                    start_time: (crate::util::Timestamp(0), crate::util::Timestamp(0)),
                    period: (RelativeDuration::minutes(10), RelativeDuration::minutes(10)),
                    ratio: 1,
                },
                0.2
            )
            .unwrap(),
            vec![(
                Timeseries {
                    tag: String::from("blindern1"),
                    values: vec![Flag::Pass, Flag::Fail, Flag::DataMissing]
                },
                Timeseries {
                    tag: String::from("blindern2"),
                    values: vec![Flag::Pass, Flag::Fail, Flag::DataMissing]
                }
            )]
        )
    }
}
