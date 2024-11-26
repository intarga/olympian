use crate::{ConsistencyCache, Flag, Timeseries, TimeseriesPair};

/// Compares a single value to a sequence, where the single value should never
/// be greater than any value in the sequence (including an adjustment)
///
/// Returns:
/// For the single value:
/// - [`Flag::DataMissing`] if the single value is missing,
/// - [`Flag::Fail`] if this invariant is broken (i.e the single value is greater than the any element
///   of the sequence plus the adjustment),
/// - [`Flag::DataMissing`] if any of the elements is missing, as we cannot be sure a missing data
///   point did not violate the invariant.
/// - [`Flag::Pass`] otherwise.
///
/// For each element in the sequence:
/// - [`Flag::DataMissing`] if the single value or the element is missing,
/// - [`Flag::Fail`] if this invariant is broken (i.e the single value is greater than the element
///   plus the adjustment),
/// - [`Flag::Pass`] otherwise.
pub fn single_greater_than_any(
    single: Option<f32>,
    sequence: &[Option<f32>],
    adjustment: f32,
) -> (Flag, Vec<Flag>) {
    let single = match single {
        Some(value) => value,
        // If the single is missing, we can't do a check at all
        None => return (Flag::DataMissing, vec![Flag::DataMissing; sequence.len()]),
    };

    let sequence_flags: Vec<Flag> = sequence
        .iter()
        // for each element of the sequence
        .map(|elem| match elem {
            Some(value) => {
                if single > value + adjustment {
                    // if the value violates the invariant, flag it as Fail
                    Flag::Fail
                } else {
                    Flag::Pass
                }
            }
            // if the value is missing, flag as DataMissing
            None => Flag::DataMissing,
        })
        .collect();

    let single_flag = if sequence_flags.iter().any(|f| *f == Flag::Fail) {
        // if the invariant was violated for any element of the sequence, it was for the single
        // value too
        Flag::Fail
    } else if sequence_flags.iter().any(|f| *f == Flag::DataMissing) {
        // else if no violation was detected, but there was missing data in the sequence, we cannot
        // say for sure that the invariant wasn't invalidated
        Flag::DataMissing
    } else {
        Flag::Pass
    };

    (single_flag, sequence_flags)
}

/// Apply [`single_greater_than_any`] to a whole [`ConsistencyCache`]
///
/// ## Panics
///
/// - `cache.ratio` is 0
pub fn single_greater_than_any_cache(
    cache: &ConsistencyCache,
    adjustment: f32,
) -> Vec<TimeseriesPair<Flag>> {
    let num_series = cache.data.len();
    let mut result_vec = Vec::with_capacity(num_series);

    for i in 0..num_series {
        let (series1, series2) = &cache.data[i];

        let (flags1, flags2): (Vec<Flag>, Vec<Vec<Flag>>) = series1
            .values
            .iter()
            .zip(series2.values.chunks(cache.ratio))
            .map(|(single, sequence)| single_greater_than_any(*single, sequence, adjustment))
            .unzip();

        result_vec.push((
            Timeseries {
                tag: series1.tag.clone(),
                values: flags1,
            },
            Timeseries {
                tag: series2.tag.clone(),
                values: flags2.concat(),
            },
        ))
    }

    result_vec
}

#[cfg(test)]
mod tests {
    use super::*;
    use chronoutil::RelativeDuration;

    #[test]
    fn test_single_less_than_max() {
        assert_eq!(
            single_greater_than_any(Some(1.), &[Some(1.), Some(2.), Some(2.)], 0.2),
            (Flag::Pass, vec![Flag::Pass, Flag::Pass, Flag::Pass])
        );
        assert_eq!(
            single_greater_than_any(Some(1.), &[Some(1.), Some(2.), Some(2.)], -0.2),
            (Flag::Fail, vec![Flag::Fail, Flag::Pass, Flag::Pass])
        );
        assert_eq!(
            single_greater_than_any(Some(1.), &[Some(1.), None, Some(2.)], -0.2),
            (Flag::Fail, vec![Flag::Fail, Flag::DataMissing, Flag::Pass])
        );
        assert_eq!(
            single_greater_than_any(Some(1.), &[Some(1.), None, Some(2.)], 0.2),
            (
                Flag::DataMissing,
                vec![Flag::Pass, Flag::DataMissing, Flag::Pass]
            )
        );
    }

    #[test]
    fn test_single_greater_than_any_cache() {
        assert_eq!(
            single_greater_than_any_cache(
                &ConsistencyCache {
                    data: vec![(
                        Timeseries {
                            tag: String::from("blindern1"),
                            values: vec![Some(1.), Some(1.4), Some(1.4), Some(1.)]
                        },
                        Timeseries {
                            tag: String::from("blindern2"),
                            values: vec![
                                Some(1.),
                                Some(2.),
                                Some(2.),
                                //-------
                                Some(1.),
                                Some(2.),
                                Some(2.),
                                //-------
                                Some(1.),
                                None,
                                Some(2.),
                                //-------
                                Some(1.),
                                None,
                                Some(2.)
                            ]
                        }
                    )],
                    start_time: (crate::util::Timestamp(0), crate::util::Timestamp(0)),
                    period: (RelativeDuration::minutes(5), RelativeDuration::minutes(15)),
                    ratio: 3,
                },
                0.2
            ),
            vec![(
                Timeseries {
                    tag: String::from("blindern1"),
                    values: vec![Flag::Pass, Flag::Fail, Flag::Fail, Flag::DataMissing]
                },
                Timeseries {
                    tag: String::from("blindern2"),
                    values: vec![
                        Flag::Pass,
                        Flag::Pass,
                        Flag::Pass,
                        //---------
                        Flag::Fail,
                        Flag::Pass,
                        Flag::Pass,
                        //---------
                        Flag::Fail,
                        Flag::DataMissing,
                        Flag::Pass,
                        //---------
                        Flag::Pass,
                        Flag::DataMissing,
                        Flag::Pass
                    ]
                }
            )]
        )
    }
}
