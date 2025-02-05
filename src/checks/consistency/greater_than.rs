use crate::{ConsistencyCache, Error, Flag, Timeseries, TimeseriesPair};

/// Consistency check between 2 climate parameters, where one (including a correction) should
/// never be greater than the other.
///
/// Returns:
/// - [`Flag::DataMissing`] if either datum is missing,
/// - [`Flag::Fail`] if datum1 + datum1_correction > datum2,
/// - [`Flag::Pass`] otherwise.
pub fn greater_than(datum1: Option<f64>, datum2: Option<f64>, datum1_correction: f64) -> Flag {
    if datum1.is_none() || datum2.is_none() {
        return Flag::DataMissing;
    }

    if datum1.unwrap() + datum1_correction > datum2.unwrap() {
        // TODO: confirm these are the right way around
        Flag::Fail
    } else {
        Flag::Pass
    }
}

/// Apply [`greater_than`] to a whole [`ConsistencyCache`]
///
/// `cache.ratio` must be 1, as that is the only alignment that makes sense for [`greater_than`]
///
/// ## Errors
///
/// - `cache.ratio` is not 1
pub fn greater_than_cache(
    cache: &ConsistencyCache,
    correction: f64,
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
            .map(|(datum1, datum2)| greater_than(*datum1, *datum2, correction))
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
    fn test_greater_than() {
        assert_eq!(greater_than(Some(1.), Some(1.), 0.2), Flag::Fail);
        assert_eq!(greater_than(Some(1.), Some(1.5), 0.2), Flag::Pass);
        assert_eq!(greater_than(Some(1.), None, 0.2), Flag::DataMissing);
    }

    #[test]
    fn test_greater_than_cache() {
        assert_eq!(
            greater_than_cache(
                &ConsistencyCache {
                    data: vec![(
                        Timeseries {
                            tag: String::from("blindern1"),
                            values: vec![Some(1.), Some(1.), Some(1.)]
                        },
                        Timeseries {
                            tag: String::from("blindern2"),
                            values: vec![Some(1.), Some(1.5), None]
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
                    values: vec![Flag::Fail, Flag::Pass, Flag::DataMissing]
                },
                Timeseries {
                    tag: String::from("blindern2"),
                    values: vec![Flag::Fail, Flag::Pass, Flag::DataMissing]
                }
            )]
        )
    }
}
