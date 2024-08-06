use crate::{Error, Flag, SeriesCache};

/// Range check over pairs of values, where each member of the pair must be in its range  to pass
///
/// If either observation in a pair is missing, Flag::DataMissing with be returned, else if either is outside the
/// upper or lower limits, Flag::Fail, else Flag::Pass.
pub fn range_check_pair(
    data1: &SeriesCache,
    upper_limit1: f32,
    lower_limit1: f32,
    data2: &SeriesCache,
    upper_limit2: f32,
    lower_limit2: f32,
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
            } else if elem1.unwrap() <= upper_limit1
                && elem1.unwrap() >= lower_limit1
                && elem2.unwrap() <= upper_limit2
                && elem2.unwrap() >= lower_limit2
            {
                Flag::Pass
            } else {
                Flag::Fail
            }
        })
        .collect())
}
