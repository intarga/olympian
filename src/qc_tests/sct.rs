use crate::{util, util::spatial_tree::SpatialPoint, Error, Flag, SpatialCache};
use faer::{solvers::SolverCore, Mat};

fn subset<T: Copy>(array: &[T], indices: &[usize]) -> Vec<T> {
    let new_length = indices.len();
    let mut new_array = Vec::with_capacity(new_length);

    for i in 0..new_length {
        new_array.push(array[indices[i]]);
    }

    new_array
}

fn compute_vertical_profile_theil_sen(
    elevs: &[f32],
    values: &[f32],
    num_min_prof: usize,
    min_elev_diff: f32,
) -> Vec<f32> {
    let n = values.len();

    // Starting value guesses
    let gamma: f32 = -0.0065;
    let mean_t: f32 = values.iter().sum::<f32>() / n as f32; // should this be f64?

    // special case when all observations have the same elevation
    if elevs.iter().min_by(|a, b| a.total_cmp(b)) == elevs.iter().max_by(|a, b| a.total_cmp(b)) {
        return vec![mean_t; n];
    }

    // Check if terrain is too flat
    let z05 = compute_quantile(0.05, elevs);
    let z95 = compute_quantile(0.95, elevs);

    // should we use the basic or more complicated vertical profile?
    let use_basic = n < num_min_prof || (z95 - z05) < min_elev_diff;

    // Theil-Sen (Median-slope) Regression (Wilks (2019), p. 284)
    let m_median = if use_basic {
        gamma
    } else {
        let nm = n * (n - 1) / 2;
        let mut m: Vec<f32> = Vec::with_capacity(nm);
        for i in 0..(n - 1) {
            for j in (i + 1)..n {
                m.push(if (elevs[i] - elevs[j]).abs() < 1. {
                    0.
                } else {
                    (values[i] - values[j]) / (elevs[i] - elevs[j])
                })
            }
        }
        compute_quantile(0.5, &m)
    };
    let q: Vec<f32> = values
        .iter()
        .zip(elevs)
        .map(|(val, elev)| val - m_median * elev)
        .collect();
    let q_median = compute_quantile(0.5, &q);

    elevs
        .iter()
        .map(|elev| q_median + m_median * elev)
        .collect()
}

// TODO: replace assertions with errors or remove them
fn compute_quantile(quantile: f32, array: &[f32]) -> f32 {
    let mut new_array: Vec<f32> = array
        .iter()
        .copied()
        .filter(|x| util::is_valid(*x))
        .collect();
    new_array.sort_by(|a, b| a.total_cmp(b));

    let n = new_array.len();

    assert!(n > 0);

    // get the quantile from the sorted array
    let lower_index = (quantile * (n - 1) as f32).floor() as usize;
    let upper_index = (quantile * (n - 1) as f32).ceil() as usize;
    let lower_value = new_array[lower_index];
    let upper_value = new_array[upper_index];
    let lower_quantile = lower_index as f32 / (n - 1) as f32;
    let upper_quantile = upper_index as f32 / (n - 1) as f32;
    let exact_q = if lower_index == upper_index {
        lower_value
    } else {
        assert!(upper_quantile > lower_quantile);
        assert!(quantile >= lower_quantile);
        let f = (quantile - lower_quantile) / (upper_quantile - lower_quantile);
        assert!(f >= 0.);
        assert!(f <= 1.);
        lower_value + (upper_value - lower_value) * f
    };

    assert!(util::is_valid(exact_q));

    exact_q
}

fn invert_matrix(input: &Mat<f32>) -> Mat<f32> {
    let lu = input.partial_piv_lu();
    lu.inverse()
}

fn remove_flagged<'a>(
    neighbours: Vec<&'a SpatialPoint>,
    distances: Vec<f32>,
    flags: &[Flag],
) -> (Vec<&'a SpatialPoint>, Vec<f32>) {
    let vec_length = neighbours.len();
    let mut neighbours_new = Vec::with_capacity(vec_length);
    let mut distances_new = Vec::with_capacity(vec_length);

    for i in 0..vec_length {
        if flags[neighbours[i].data] == Flag::Pass {
            neighbours_new.push(neighbours[i]);
            distances_new.push(distances[i]);
        }
    }

    (neighbours_new, distances_new)
}

/// Spatial QC test that compares an observation to an expected value generated from it's
/// neighbours, taking their distance and elevation into account.
///
/// The SCT compares each observation to what is expected given the other observations in the nearby
/// area. If the deviation is large, the observation is removed. The SCT uses optimal interpolation
/// (OI) to compute an expected value for each observation. The background for the OI is computed
/// from a general vertical profile of observations in the area.
///
/// When a given observation is being processed, the `outer_radius` \[m\] defines which other
/// observations will be used to determine if the observation should be flagged or not. This can be
/// computationally expensive, if a new circle of observation is used when processing the next
/// observation. To save time, the calculations used for one observation can be reused for all
/// other observations within the `inner_radius` \[m\].
///
/// The test will only be performed if there are at least `num_min` observations inside the outer
/// circle. Also, to reduce computation time, only the nearest `num_max` observations will be used
/// in the outer circle, even if there are more available observations. The SCT inverts a matrix
/// with the same dimensions as the number of available observations, so preventing really
/// large matrices in observation dense areas significantly lowers computation times.
///
/// The thresholds for determining if an observation is removed is set by `pos` and `neg`. `pos`
/// sets the number of standard deviations above the expected value a given observation is allowed
/// before being flagged. Similarly, `neg` is used for negative deviations. Different deviations
/// for positive and negative are useful for sun-exposed temperature sensors in cold inversion
/// conditions, where large negative deviations are more likely to be valid than positive ones.
///
/// An adaptive horizontal decorrelation length is determined automatically, however a minimum
/// allowed value can be set by `dhmin` \[m\]. The vertical decorrelation lengthscale is set by `dz`
/// \[m\].
///
/// The background for the OI is computed by finding a suitable vertical profile of the
/// observations in the outer circle. `dzmin` \[m\] sets the minimum elevation range required to
/// compute a vertical profile.
///
/// `num_iterations` specifies how many sweeps of all observations will be performed. Observations
/// removed in earlier iterations will not be used in the calculations in later iterations.
///
///  ![Image](https://github.com/metno/titanlib/wiki/images/sct.png)
///
/// ## Input parameters
///
/// | Parameter            | Unit | Description |
/// | -------------------- | ---- | ----------- |
/// | data                 | N/A  | See [`SpatialCache`] |
/// | num_min              | N/A  | If an observation has fewer neighbours than this it will not be QCed |
/// | num_max              | N/A  | A cap on the number of neighbours used to compute the expected value |
/// | inner_radius         | m    | Radius in which OI will be reused |
/// | outer_radius         | m    | Radius for computing OI and background |
/// | num_iterations       | N/A  | The number of iterations to perform |
/// | num_min_prof         | N/A  | Minimum number of observations to compute vertical profile |
/// | min_elev_diff        | m    | Minimum elevation difference to compute vertical profile |
/// | min_horizontal_scale | m    | Minimum horizontal decorrelation length |
/// | vertical_scale       | m    | Vertical decorrelation length |
/// | pos                  | σ    | Positive deviation allowed |
/// | neg                  | σ    | Negative deviation allowed |
/// | eps2                 | N/A  | Ratio of observation error variance to background variance |
/// | obs_to_check*        | N/A  | Observations that will be checked. true=check the corresponding observation. Unchecked observations will be used to QC others, but will not be QCed themselves |
///
/// \* optional, ou = Unit of the observation, σ = Standard deviations
#[allow(clippy::too_many_arguments)]
pub fn sct(
    data: &SpatialCache,
    num_min: usize,
    num_max: usize,
    inner_radius: f32,
    outer_radius: f32,
    num_iterations: u32,
    num_min_prof: usize,
    min_elev_diff: f32,
    min_horizontal_scale: f32,
    vertical_scale: f32,
    pos: &[f32],
    neg: &[f32],
    eps2: &[f32],
    obs_to_check: Option<&[bool]>,
) -> Result<Vec<Flag>, Error> {
    let vec_length = data.values.len();

    // should we check lats, lons, etc. individually?
    // move to constructor?
    if data.rtree.tree.size() != vec_length {
        return Err(Error::InvalidInputShape(String::from("tree_points")));
    }
    if pos.len() != vec_length {
        return Err(Error::InvalidInputShape(String::from("pos")));
    }
    if neg.len() != vec_length {
        return Err(Error::InvalidInputShape(String::from("neg")));
    }
    if eps2.len() != vec_length {
        return Err(Error::InvalidInputShape(String::from("eps2")));
    }
    if let Some(obs_to_check_inner) = obs_to_check {
        if obs_to_check_inner.len() != vec_length {
            return Err(Error::InvalidInputShape(String::from("obs_to_check")));
        }
    }
    if num_min < 2 {
        return Err(Error::InvalidArg(
            String::from("num_min"),
            String::from("must be > 1"),
        ));
    }
    if num_max < num_min {
        return Err(Error::InvalidArg(
            String::from("num_max"),
            String::from("must be > num_min"),
        ));
    }
    if num_iterations < 1 {
        return Err(Error::InvalidArg(
            String::from("num_iterations"),
            String::from("must be >= 1"),
        ));
    }
    if min_elev_diff <= 0. {
        return Err(Error::InvalidArg(
            String::from("min_elev_diff"),
            String::from("must be > 0"),
        ));
    }
    if min_horizontal_scale <= 0. {
        return Err(Error::InvalidArg(
            String::from("min_horizontal_scale"),
            String::from("must be > 0"),
        ));
    }
    if vertical_scale <= 0. {
        return Err(Error::InvalidArg(
            String::from("vertical_scale"),
            String::from("must be > 0"),
        ));
    }
    if inner_radius < 0. {
        return Err(Error::InvalidArg(
            String::from("inner_radius"),
            String::from("must be >= 0"),
        ));
    }
    if outer_radius < inner_radius {
        return Err(Error::InvalidArg(
            String::from("outer_radius"),
            String::from("must be >= inner_radius"),
        ));
    }
    for i in 0..vec_length {
        if eps2[i] <= 0. {
            return Err(Error::InvalidArg(
                String::from("eps2"),
                String::from("all values must be > 0"),
            ));
        }
        if pos[i] < 0. {
            return Err(Error::InvalidArg(
                String::from("pos"),
                String::from("all values must be >= "),
            ));
        }
        if neg[i] < 0. {
            return Err(Error::InvalidArg(
                String::from("neg"),
                String::from("all values must be >= 0"),
            ));
        }
    }

    let mut flags = vec![Flag::Pass; vec_length];
    let mut prob_gross_error = vec![0.; vec_length];

    for (flag, elev) in flags.iter_mut().zip(data.rtree.elevs.iter()) {
        if !util::is_valid(*elev) {
            *flag = Flag::Invalid;
        }
    }

    // would it make more sense for this to be a 1-based index?
    for _iteration in 0..num_iterations {
        // resets each loop, for breaking if we don't throw anything new out
        let mut num_thrown_out: u32 = 0;

        // keep track of which observations have been checked
        let mut checked = vec![false; vec_length];

        for i in 0..vec_length {
            if let Some(obs_to_check_inner) = obs_to_check {
                if !obs_to_check_inner[i] {
                    checked[i] = true;
                    continue;
                }
            }

            // continue if station is already flagged
            if flags[i] != Flag::Pass {
                checked[i] = true;
                continue;
            }
            if checked[i] {
                continue;
            }

            let (neighbours_unfiltered, distances_unfiltered) =
                data.rtree.get_neighbours_with_distance(
                    data.rtree.lats[i],
                    data.rtree.lons[i],
                    outer_radius,
                    true,
                );
            let (mut neighbours, mut distances) =
                remove_flagged(neighbours_unfiltered, distances_unfiltered, &flags);

            if neighbours.len() > num_max {
                let mut pairs: Vec<(&SpatialPoint, f32)> =
                    neighbours.into_iter().zip(distances.into_iter()).collect();
                pairs.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));

                (neighbours, distances) = pairs.into_iter().take(num_max).unzip()
            }
            if neighbours.len() < num_min {
                checked[i] = true;
                flags[i] = Flag::Isolated;
                continue;
            }

            let box_size = neighbours.len();
            let neighbour_indices: Vec<usize> =
                neighbours.into_iter().map(|point| point.data).collect();

            // call SCT on this box of values
            let lats_box = subset(&data.rtree.lats, &neighbour_indices);
            let lons_box = subset(&data.rtree.lons, &neighbour_indices);
            let elevs_box = subset(&data.rtree.elevs, &neighbour_indices);
            let values_box = subset(data.data(), &neighbour_indices);
            let eps2_box = subset(eps2, &neighbour_indices);

            // compute the background
            // TODO: investigate why titanlib allowed negative num_min_prof
            let vertical_profile = compute_vertical_profile_theil_sen(
                &elevs_box,
                &values_box,
                num_min_prof,
                min_elev_diff,
            );

            let disth: Mat<f32> = Mat::from_fn(box_size, box_size, |i, j| {
                // TODO: remove this unwrap
                util::calc_distance(lats_box[i], lons_box[i], lats_box[j], lons_box[j]).unwrap()
            });
            let distz: Mat<f32> = Mat::from_fn(box_size, box_size, |i, j| {
                (elevs_box[i] - elevs_box[j]).abs()
            });
            // TODO: remove dh, and just reduce straight into dh_mean?
            let dh: Vec<f32> = (0..box_size)
                .map(|i| {
                    let mut dh_vector = Vec::with_capacity(box_size - 1);
                    for j in 0..box_size {
                        if i != j {
                            dh_vector.push(disth.read(i, j));
                        }
                    }
                    compute_quantile(0.10, &dh_vector)
                })
                .collect();

            let dh_mean: f32 =
                min_horizontal_scale.max(dh.into_iter().sum::<f32>() / box_size as f32);

            let mut s: Mat<f32> = Mat::from_fn(box_size, box_size, |i, j| {
                let value = (-0.5 * (disth.read(i, j) / dh_mean).powi(2)
                    - 0.5 * (distz.read(i, j) / vertical_scale).powi(2))
                .exp();
                // weight the diagonal?? (0.5 default)
                if i == j {
                    value + eps2_box[i]
                } else {
                    value
                }
            });

            // difference between actual temp and temp from vertical profile
            let d: Vec<f32> = (0..box_size)
                .map(|i| values_box[i] - vertical_profile[i])
                .collect();

            /* ---------------------------------------------------
            Beginning of real SCT
            ------------------------------------------------------*/

            // TODO: investigate case of uninvertible (singular) matrices
            let s_inv = invert_matrix(&s);

            // unweight the diagonal
            for (i, eps2) in eps2_box.iter().enumerate() {
                s.write(i, i, s.read(i, i) - eps2)
            }

            let s_inv_d: Vec<f32> = (0..box_size)
                .map(|i| (0..box_size).map(|j| s_inv.read(i, j) * d[j]).sum())
                .collect();

            let ares_temp: Vec<f32> = (0..box_size)
                .map(|i| (0..box_size).map(|j| s.read(i, j) * s_inv_d[j]).sum())
                .collect();

            let z_inv: Vec<f32> = (0..box_size).map(|i| 1. / s_inv.read(i, i)).collect();

            let ares: Vec<f32> = (0..box_size).map(|i| ares_temp[i] - d[i]).collect();

            let cvres: Vec<f32> = (0..box_size).map(|i| -1. * z_inv[i] * s_inv_d[i]).collect();

            let sig2o = 0.01_f32
                .max((0..box_size).map(|i| d[i] * -1. * ares[i]).sum::<f32>() / box_size as f32);

            let curr = i;
            for i in 0..box_size {
                let index = neighbour_indices[i];
                if let Some(obs_to_check_inner) = obs_to_check {
                    if !obs_to_check_inner[index] {
                        checked[curr] = true;
                        continue;
                    }
                }
                let dist = distances[i];
                if dist <= inner_radius {
                    let pog: f32 = cvres[i] * ares[i] / sig2o;
                    assert!(util::is_valid(pog));
                    prob_gross_error[index] = pog.max(prob_gross_error[index]);
                    if (cvres[i] < 0. && pog > pos[index]) || (cvres[i] >= 0. && pog > neg[index]) {
                        flags[index] = Flag::Fail;
                        num_thrown_out += 1;
                    }
                    checked[index] = true;
                }
            }
        }

        if num_thrown_out == 0 {
            break;
        }
    }

    Ok(flags)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sct_simple() {
        assert_eq!(
            sct(
                &SpatialCache::new(
                    [60.; 3].to_vec(),
                    [10., 10.01, 10.02].to_vec(),
                    [0.; 3].to_vec(),
                    [0., 1., 100.].to_vec(),
                ),
                3,
                10,
                10000.,
                10000.,
                1,
                0,
                100.,
                10000.,
                200.,
                &[2.; 3],
                &[2.; 3],
                &[0.5; 3],
                None
            )
            .unwrap(),
            [Flag::Pass, Flag::Pass, Flag::Fail]
        );

        const N: usize = 10000;
        assert_eq!(
            sct(
                &SpatialCache::new(
                    (0..N).map(|i| ((i as f32).powi(2) * 0.001) % 1.).collect(),
                    (0..N)
                        .map(|i| ((i as f32 + 1.).powi(2) * 0.001) % 1.)
                        .collect(),
                    vec![1.; N],
                    vec![1.; N],
                ),
                5,
                100,
                50000.,
                150000.,
                5,
                20,
                200.,
                10000.,
                200.,
                &vec![4.; N],
                &vec![8.; N],
                &vec![0.5; N],
                Some(&vec![true; N])
            )
            .unwrap(),
            vec![Flag::Pass; N]
        );
    }
}
