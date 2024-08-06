use crate::{Error, Flag, SeriesCache};

/// Consistency check that fails if the first element is under a limit, while the second is a
/// special value.
///
/// typically used to compare cloud area fraction with high type cloud
pub fn lower_limit_special_value_pair(
    data1: &SeriesCache,
    limit: f32,
    data2: &SeriesCache,
    special_value: f32,
) -> Result<Vec<Flag>, Error> {
    let trimmed1 = &data1.values[data1.num_leading_points as usize
        ..(data1.values.len() - data1.num_trailing_points as usize)];
    let trimmed2 = &data2.values[data2.num_leading_points as usize
        ..(data2.values.len() - data2.num_trailing_points as usize)];

    if trimmed1.len() != trimmed2.len() {
        return Err(Error::InvalidInputShape(
            "data1 and data2 must have the same dimensions".to_string(),
        ));
    }

    let windows = trimmed1.iter().zip(trimmed2);

    Ok(windows
        .map(|(elem1, elem2)| {
            if elem1.is_none() || elem2.is_none() {
                Flag::DataMissing
            } else if elem1.unwrap() < limit && elem2.unwrap() == special_value {
                Flag::Fail
            } else {
                Flag::Pass
            }
        })
        .collect())
}
