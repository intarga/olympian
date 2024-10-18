use crate::{
    util::{self, spatial_tree::SpatialTree, SingleOrVec},
    DataCache, Error, Flag,
};

/// Specific arguments to buddy_check, broken into a struct to make the function
/// signature more readable.
#[derive(Debug, Clone)]
pub struct BuddyCheckArgs {
    /// Search radius in which buddies of an observation will be found. Unit: m
    pub radii: SingleOrVec<f32>,
    /// The minimum buddies an observation can have (lest it be flagged [`Flag::Isolated`])
    pub nums_min: SingleOrVec<u32>,
    /// The variance threshold for flagging a station. Unit: σ (standard deviations)
    pub threshold: f32,
    /// The maximum difference in elevation for a buddy (if negative will not check for height
    /// difference). Unit: m
    pub max_elev_diff: f32,
    /// Linear elevation gradient with height. Unit: ou/m (ou = unit of observation)
    pub elev_gradient: f32,
    /// If the standard deviation of values in a neighborhood are less than min_std, min_std will
    /// be used instead
    pub min_std: f32,
    /// The number of iterations of buddy_check to perform before returning
    pub num_iterations: u32,
}

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
pub fn buddy_check(
    data: &[Option<f32>],
    rtree: &SpatialTree,
    args: &BuddyCheckArgs,
    obs_to_check: Option<&[bool]>,
) -> Result<Vec<Flag>, Error> {
    // TODO: Check input vectors are properly sized

    let mut flags: Vec<Flag> = data
        .iter()
        .map(|opt| match opt {
            Some(value) => {
                if util::is_valid(*value) {
                    Flag::Pass
                } else {
                    Flag::Fail
                }
            }
            None => Flag::DataMissing,
        })
        .collect();

    let mut num_removed_last_iteration = 0;

    for _iteration in 1..=args.num_iterations {
        for i in 0..data.len() {
            if flags[i] != Flag::Pass {
                continue;
            }

            if obs_to_check.map_or(true, |inner| inner[i]) {
                let (lat, lon, elev) = rtree.get_coords_at_index(i);
                let neighbours = rtree.get_neighbours(lat, lon, *args.radii.index(i), false);

                let mut list_buddies: Vec<f32> = Vec::new();

                if neighbours.len() >= *args.nums_min.index(i) as usize {
                    for neighbour in neighbours {
                        let (_, _, neighbour_elev) = rtree.get_coords_at_index(neighbour.data);

                        if flags[neighbour.data] != Flag::Pass {
                            continue;
                        }

                        if args.max_elev_diff > 0.0 {
                            let elev_diff = elev - neighbour_elev;

                            if elev_diff.abs() <= args.max_elev_diff {
                                let adjusted_value =
                                    // safe to unwrap because if this was none, we would have `continue;`d
                                    // when checking the flags
                                    data[neighbour.data].unwrap() + (elev_diff * args.elev_gradient);

                                list_buddies.push(adjusted_value);
                            }
                        } else {
                            // same here
                            list_buddies.push(data[neighbour.data].unwrap());
                        }
                    }
                }

                if list_buddies.len() >= *args.nums_min.index(i) as usize {
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
                        args.min_std,
                        |x, y| x.partial_cmp(y).unwrap_or(std::cmp::Ordering::Equal),
                    );

                    // and same here
                    if (data[i].unwrap() - mean).abs() / std_adjusted > args.threshold {
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

/// Apply [`buddy_check`] to a whole [`DataCache`]
pub fn buddy_check_cache(
    cache: &DataCache,
    args: &BuddyCheckArgs,
    // TODO: should we allow different obs_to_check for each timeslice?
    obs_to_check: Option<&[bool]>,
) -> Result<Vec<(String, Vec<Flag>)>, Error> {
    let series_len = cache.data[0].1.len();

    let mut result_vec: Vec<(String, Vec<Flag>)> = cache
        .data
        .iter()
        .map(|ts| (ts.0.clone(), Vec::with_capacity(series_len)))
        .collect();

    for i in (cache.num_leading_points as usize)..(series_len - cache.num_trailing_points as usize)
    {
        let timeslice: Vec<Option<f32>> = cache.data.iter().map(|v| v.1[i]).collect();
        let spatial_result = buddy_check(&timeslice, &cache.rtree, args, obs_to_check)?;

        for i in 0..spatial_result.len() {
            result_vec[i].1.push(spatial_result[i]);
        }
    }

    Ok(result_vec)
}

#[cfg(test)]
mod tests {
    use super::*;

    const BUDDY_N: usize = 10;
    #[test]
    fn test_buddy_check() {
        assert_eq!(
            buddy_check(
                &[
                    Some(0.),
                    Some(0.),
                    Some(0.),
                    Some(0.),
                    Some(0.),
                    Some(0.),
                    Some(0.),
                    Some(0.),
                    Some(0.1),
                    Some(1.)
                ]
                .to_vec(),
                &SpatialTree::from_latlons(
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
                ),
                &BuddyCheckArgs {
                    radii: SingleOrVec::Single(10000.),
                    nums_min: SingleOrVec::Single(1),
                    threshold: 1.,
                    max_elev_diff: 200.,
                    elev_gradient: -0.0065,
                    min_std: 0.01,
                    num_iterations: 2,
                },
                None,
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
