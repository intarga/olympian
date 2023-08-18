use crate::{util, util::spatial_tree::SpatialPoint, Error, Flag, SpatialTree};
use dyn_stack::{DynStack, GlobalMemBuffer};
use faer_core::{Mat, Parallelism};
use faer_lu::partial_pivoting::{
    compute::{lu_in_place, lu_in_place_req},
    inverse::{invert, invert_req},
};
use reborrow::Reborrow;
use std::iter::repeat;

fn subset<T: Copy>(array: &[T], indices: &[usize]) -> Vec<T> {
    let new_length = indices.len();
    let mut new_array = Vec::with_capacity(new_length);

    for i in 0..new_length {
        new_array.push(array[indices[i]]);
    }

    new_array
}

fn compute_vertical_profile_theil_sen(
    elevs: &Vec<f32>,
    values: &Vec<f32>,
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
    let n = input.nrows();

    let mut lu = input.clone();
    let mut row_perm: Vec<usize> = repeat(0).take(n).collect();
    let mut row_perm_inv = row_perm.clone();

    let (_, row_perm) = lu_in_place(
        lu.as_mut(),
        &mut row_perm,
        &mut row_perm_inv,
        // TODO: can we give a better parallelism hint?
        Parallelism::Rayon(0),
        DynStack::new(&mut GlobalMemBuffer::new(
            // TODO: do something about this unwrap
            lu_in_place_req::<f32>(n, n, Parallelism::Rayon(0), Default::default()).unwrap(),
        )),
        Default::default(),
    );

    let mut inv = Mat::zeros(n, n);
    invert(
        inv.as_mut(),
        lu.as_ref(),
        row_perm.rb(),
        Parallelism::Rayon(0),
        DynStack::new(&mut GlobalMemBuffer::new(
            invert_req::<f32>(n, n, Parallelism::Rayon(0)).unwrap(),
        )),
    );

    inv
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

#[allow(clippy::too_many_arguments)]
pub fn sct(
    tree_points: &SpatialTree,
    values: &[f32],
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
    let vec_length = values.len();

    // should we check lats, lons, etc. individually?
    if tree_points.tree.size() != vec_length {
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

    for (flag, elev) in flags.iter_mut().zip(tree_points.elevs.iter()) {
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

            let (neighbours_unfiltered, distances_unfiltered) = tree_points
                .get_neighbours_with_distance(
                    tree_points.lats[i],
                    tree_points.lons[i],
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
            let lats_box = subset(&tree_points.lats, &neighbour_indices);
            let lons_box = subset(&tree_points.lons, &neighbour_indices);
            let elevs_box = subset(&tree_points.elevs, &neighbour_indices);
            let values_box = subset(values, &neighbour_indices);
            let eps2_box = subset(eps2, &neighbour_indices);

            // compute the background
            // TODO: investigate why titanlib allowed negative num_min_prof
            let vertical_profile = compute_vertical_profile_theil_sen(
                &elevs_box,
                &values_box,
                num_min_prof,
                min_elev_diff,
            );

            let disth: Mat<f32> = Mat::with_dims(box_size, box_size, |i, j| {
                util::calc_distance(lats_box[i], lons_box[i], lats_box[j], lons_box[j])
            });
            let distz: Mat<f32> = Mat::with_dims(box_size, box_size, |i, j| {
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

            let mut s: Mat<f32> = Mat::with_dims(box_size, box_size, |i, j| {
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
                &SpatialTree::from_latlons(
                    [60.; 3].to_vec(),
                    [10., 10.01, 10.02].to_vec(),
                    [0.; 3].to_vec(),
                ),
                &[0., 1., 100.],
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
        )
    }
}
