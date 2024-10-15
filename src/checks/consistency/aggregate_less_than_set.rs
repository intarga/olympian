use crate::Flag;

// TODO: this test seems poorly considered
// TODO: add more docs
/// Compares a minimum aggregated value to a higher resolution set, to ensure it is not less than
/// the minimum of the set.
pub fn aggregate_less_than_set(
    aggregate: Option<f32>,
    set: &[Option<f32>],
    adjustment: f32,
) -> (Flag, Vec<Flag>) {
    if aggregate.is_none() || set.iter().all(|elem| elem.is_none()) {
        return (Flag::DataMissing, vec![Flag::DataMissing; set.len()]);
    }

    let (min_index, min) = set
        .iter()
        .enumerate()
        .filter_map(|elem| elem.1.map(|inner| (elem.0, inner)))
        .reduce(|acc, e| if acc.1 < e.1 { acc } else { e })
        .unwrap(); // this unwrap is safe, since we already checked set contains a Some

    let mut set_flags: Vec<Flag> = set
        .iter()
        .map(|elem| {
            if elem.is_none() {
                Flag::DataMissing
            } else {
                Flag::Pass
            }
        })
        .collect();

    if aggregate.unwrap() < min + adjustment {
        set_flags[min_index] = Flag::Fail;
        (Flag::Fail, set_flags)
    } else {
        (Flag::Pass, set_flags)
    }
}
