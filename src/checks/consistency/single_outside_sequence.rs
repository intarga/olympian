use crate::{ConsistencyCache, Flag, Timeseries, TimeseriesPair};

/// Compares a single value to a higher resolution sequence, where the single value should never
/// be outside the range of the sequence (including an adjustment)
///
/// Returns:
/// - [`Flag::DataMissing`] if the single value is missing,
/// - [`Flag::Pass`] if this invariant is held (i.e the single value is greater than the minimum
///   of the sequence plus the adjustment AND less than the maximum),
/// - [`Flag::DataMissing`] if any of the elements is missing, as we cannot be sure a missing data
///   point did not satisfy the invariant.
/// - [`Flag::Fail`] otherwise.
pub fn single_outside_sequence(
    single: Option<f32>,
    sequence: &[Option<f32>],
    adjustment: f32,
) -> Flag {
    let single = match single {
        Some(value) => value,
        None => {
            // If the single is missing, we can't do a check at all
            return Flag::DataMissing;
        }
    };

    // find the minimum value and maximum values in the sequence (None if they are all missing), as well as a bool
    // indicating if any value was missing
    let (minmax, missing) = sequence.iter().fold(
        (None, false),
        |acc: (Option<(f32, f32)>, bool), elem| match elem {
            // if the element wasn't missing...
            Some(value) => match acc.0 {
                // set the min if there isn't already a min, or the element is lower
                Some(minmax) => (Some((minmax.0.min(*value), minmax.1.max(*value))), acc.1),
                None => (Some((*value, *value)), acc.1),
            },
            // if the element was missing, leave the min unchanged, but set `missing` to true
            None => (acc.0, true),
        },
    );
    let (min, max) = match minmax {
        Some(value) => value,
        // if min is None at this point, then all the elements of the sequence were missing...
        None => {
            // so we can't perform the check
            return Flag::DataMissing;
        }
    };

    if single > min + adjustment && single < max + adjustment {
        // if this condition evaluates to true, the invariant held, i.e the single value is
        // definitely within the sequence
        Flag::Pass
    } else if missing {
        // if the condition was not met, but there was missing data in the sequence, we cannot say
        // for sure that the invariant was invalidated
        Flag::DataMissing
    } else {
        Flag::Fail
    }
}

/// Apply [`single_outside_sequence`] to a whole [`ConsistencyCache`]
///
/// ## Panics
///
/// - `cache.ratio` is 0
pub fn single_outside_sequence_cache(
    cache: &ConsistencyCache,
    adjustment: f32,
) -> Vec<TimeseriesPair<Flag>> {
    let num_series = cache.data.len();
    let mut result_vec = Vec::with_capacity(num_series);

    for i in 0..num_series {
        let (series1, series2) = &cache.data[i];

        let flags1: Vec<Flag> = series1
            .values
            .iter()
            .zip(series2.values.chunks(cache.ratio))
            .map(|(datum1, datum2)| single_outside_sequence(*datum1, datum2, adjustment))
            .collect();

        // since single_outside_sequence only returns one flag that applies to both the single
        // and the whole sequence, we need to duplicate each of these `cache.ratio` times to
        // cover the whole sequence
        let flags2: Vec<Flag> = flags1
            .iter()
            .flat_map(|flag| std::iter::repeat_n(*flag, cache.ratio))
            .collect();

        result_vec.push((
            Timeseries {
                tag: series1.tag.clone(),
                values: flags1,
            },
            Timeseries {
                tag: series2.tag.clone(),
                values: flags2,
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
    fn test_single_outside_sequence() {
        assert_eq!(
            single_outside_sequence(Some(1.), &[Some(1.), Some(2.), Some(2.)], -0.2),
            Flag::Pass
        );
        assert_eq!(
            single_outside_sequence(Some(1.), &[Some(1.), Some(2.), Some(2.)], 0.2),
            Flag::Fail
        );
        assert_eq!(
            single_outside_sequence(Some(2.), &[Some(1.), Some(2.), Some(2.)], -0.2),
            Flag::Fail
        );
        assert_eq!(
            single_outside_sequence(Some(2.), &[Some(1.), Some(2.), Some(2.)], 0.2),
            Flag::Pass
        );
        assert_eq!(
            single_outside_sequence(Some(1.), &[Some(1.), None, Some(2.)], -0.2),
            Flag::Pass
        );
        assert_eq!(
            single_outside_sequence(Some(1.), &[Some(1.), None, Some(2.)], 0.2),
            Flag::DataMissing
        );
    }

    #[test]
    fn test_single_outside_sequence_cache() {
        assert_eq!(
            single_outside_sequence_cache(
                &ConsistencyCache {
                    data: vec![(
                        Timeseries {
                            tag: String::from("blindern1"),
                            values: vec![
                                Some(1.4),
                                Some(1.),
                                Some(2.4),
                                Some(2.),
                                Some(1.4),
                                Some(1.)
                            ]
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
                                Some(2.),
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
                    values: vec![
                        Flag::Pass,
                        Flag::Fail,
                        Flag::Fail,
                        Flag::Pass,
                        Flag::Pass,
                        Flag::DataMissing
                    ]
                },
                Timeseries {
                    tag: String::from("blindern2"),
                    values: vec![
                        Flag::Pass,
                        Flag::Pass,
                        Flag::Pass,
                        //---------
                        Flag::Fail,
                        Flag::Fail,
                        Flag::Fail,
                        //---------
                        Flag::Fail,
                        Flag::Fail,
                        Flag::Fail,
                        //---------
                        Flag::Pass,
                        Flag::Pass,
                        Flag::Pass,
                        //---------
                        Flag::Pass,
                        Flag::Pass,
                        Flag::Pass,
                        //---------
                        Flag::DataMissing,
                        Flag::DataMissing,
                        Flag::DataMissing,
                        //---------
                    ]
                }
            )]
        )
    }
}
