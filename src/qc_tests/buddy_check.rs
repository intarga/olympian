use crate::{util, Error, Flag, SpatialCache};

/// Spatial QC test that compares an observation against its neighbours (i.e buddies) and flags
/// outliers.
///
/// The check looks for buddies of an observation (at index i) in a neighbourhood specified by
/// `radii[i]` \[m\], which is the radius of a circle around the observation to be checked. A minimum
/// number of observations (`nums_min[i]`) is required to be available inside the circle and the
/// range of elevations in the circle must not exceed `max_elev_diff` meters . The number of
/// iterations is set by `num_iterations`.
///
/// The buddy check flags observations if the (absolute value of the) difference between the
/// observations and the average of the neighbours normalized by the standard deviation in the
/// circle is greater than a predefined `threshold`. If the standard deviation of values in the
/// neighbourhood is less than `min_std`, then a value of `min_std` is used instead. `min_std`
/// should be roughly equal to the standard deviation of the error of a typical observation. If it
/// is too low, then too many observations will be flaged in areas where the variability is low.
///
/// In the case of temperature, elevation differences should be taken into account because all
/// observations are reported to the elevation of the centroid observation before averaging. A
/// linear vertical rate of change of temperature can be set by `elev_gradient`. A recommended
/// value is `elev_gradient=-0.0065` &deg;C/m (as defined in the ICAO international standard
/// atmosphere). If `max_elev_diff` is negative then elevation difference is not checked and the
/// observed values are not corrected.
///
/// It is possible to specify an optional vector `obs_to_check` to specify whether an observation
/// should be checked. The length of `obs_to_check` must be the same as the vector with the values
/// to check. The buddy check is performed only for values where the corresponding `obs_to_check`
/// element is set to true, while all values are always used as buddies for checking the data
/// quality.
///
/// ## Input parameters
///
/// | Parameter      | Unit | Description |
/// | -------------- | ---- | ----------- |
/// | data           | N/A  | See [`SpatialCache`] |
/// | radii          | m    | Search radius |
/// | nums_min       | N/A  | The minimum number of buddies a station can have |
/// | threshold      | σ    | the variance threshold for flagging a station |
/// | max_elev_diff  | m    | the maximum difference in elevation for a buddy (if negative will not check for heigh difference) |
/// | elev_gradient  | ou/m | linear elevation gradient with height |
/// | min_std        | N/A  | If the standard deviation of values in a neighborhood are less than min_std, min_std will be used instead |
/// | num_iterations | N/A  | The number of iterations to perform |
/// | obs_to_check*  | N/A  | Observations that will be checked. true=check the corresponding observation. Unchecked observations will be used to QC others, but will not be QCed themselves |
///
/// \* optional, ou = Unit of the observation, σ = Standard deviations
#[allow(clippy::too_many_arguments)]
pub fn buddy_check(
    data: &SpatialCache,
    radii: &[f32],
    nums_min: &[u32],
    threshold: f32,
    max_elev_diff: f32,
    elev_gradient: f32,
    min_std: f32,
    num_iterations: u32,
    obs_to_check: &[bool],
) -> Result<Vec<Flag>, Error> {
    // TODO: Check input vectors are properly sized

    let mut flags: Vec<Flag> = data
        .values
        .iter()
        .map(|v| {
            if util::is_valid(*v) {
                Flag::Pass
            } else {
                Flag::Fail
            }
        })
        .collect();

    let check_all = obs_to_check.len() != data.values.len();

    let mut num_removed_last_iteration = 0;

    for _iteration in 1..=num_iterations {
        for i in 0..data.values.len() {
            let radius = if radii.len() == 1 { radii[0] } else { radii[i] };
            let num_min = if nums_min.len() == 1 {
                nums_min[0]
            } else {
                nums_min[i]
            };

            if flags[i] != Flag::Pass {
                continue;
            }

            if check_all || obs_to_check[i] {
                let (lat, lon, elev) = data.rtree.get_coords_at_index(i);
                let neighbours = data.rtree.get_neighbours(lat, lon, radius, false);

                let mut list_buddies: Vec<f32> = Vec::new();

                if neighbours.len() >= num_min as usize {
                    for neighbour in neighbours {
                        let (_, _, neighbour_elev) = data.rtree.get_coords_at_index(neighbour.data);

                        if flags[neighbour.data] != Flag::Pass {
                            continue;
                        }

                        if max_elev_diff > 0.0 {
                            let elev_diff = elev - neighbour_elev;

                            if elev_diff.abs() <= max_elev_diff {
                                let adjusted_value =
                                    data.values[neighbour.data] + (elev_diff * elev_gradient);

                                list_buddies.push(adjusted_value);
                            }
                        } else {
                            list_buddies.push(data.values[neighbour.data]);
                        }
                    }
                }

                if list_buddies.len() >= num_min as usize {
                    let mean: f32 = list_buddies.iter().sum::<f32>() / list_buddies.len() as f32;
                    let variance: f32 = (list_buddies.iter().map(|x| x.powi(2)).sum::<f32>()
                        / list_buddies.len() as f32)
                        - mean.powi(2); // TODO: use a better variance algorithm?
                                        // let std = variance.sqrt();
                                        // let std_adjusted = (variance + variance / list_buddies.len() as f32).sqrt();
                                        // if std_adjusted < min_std {
                                        //     std_adjusted = min_std
                                        // }
                    let std_adjusted = std::cmp::max_by(
                        (variance + variance / list_buddies.len() as f32).sqrt(),
                        min_std,
                        |x, y| x.partial_cmp(y).unwrap_or(std::cmp::Ordering::Equal),
                    );

                    if (data.values[i] - mean).abs() / std_adjusted > threshold {
                        flags[i] = Flag::Fail;
                    }
                }
            }
        }

        let num_removed: u32 = flags
            .iter()
            .fold(0, |acc, flag| acc + (*flag != Flag::Pass) as u32);
        let num_removed_current_iteration = num_removed - num_removed_last_iteration;

        if num_removed_current_iteration == 0 {
            break;
        }

        num_removed_last_iteration = num_removed_current_iteration;
    }

    Ok(flags)
}

#[cfg(test)]
mod tests {
    use super::*;

    const BUDDY_N: usize = 10;
    #[test]
    fn test_buddy_check() {
        assert_eq!(
            buddy_check(
                &SpatialCache::new(
                    [60.; BUDDY_N].to_vec(),
                    [
                        60.,
                        60.00011111,
                        60.00022222,
                        60.00033333,
                        60.00044444,
                        60.00055556,
                        60.00066667,
                        60.00077778,
                        60.00088889,
                        60.001,
                    ]
                    .to_vec(),
                    [0.; BUDDY_N].to_vec(),
                    [0., 0., 0., 0., 0., 0., 0., 0., 0.1, 1.].to_vec()
                ),
                &[10000.],
                &[1],
                1.,
                200.,
                -0.0065,
                0.01,
                2,
                &[true; BUDDY_N],
            )
            .unwrap(),
            [
                Flag::Pass,
                Flag::Pass,
                Flag::Pass,
                Flag::Pass,
                Flag::Pass,
                Flag::Pass,
                Flag::Pass,
                Flag::Pass,
                Flag::Fail,
                Flag::Fail
            ]
        )
    }
}
