use faer_core::{MatMut, MatRef};
use faer_lu::partial_pivoting::{
    compute::{lu_in_place, lu_in_place_req},
    inverse::{invert, invert_req},
};

pub const RADIUS_EARTH: f32 = 6371.0;

pub fn is_valid(value: f32) -> bool {
    !f32::is_nan(value) && !f32::is_infinite(value)
}

pub fn subset<T: Copy>(array: &Vec<T>, indices: &Vec<usize>) -> Vec<T> {
    let new_length = indices.len();
    let mut new_array = Vec::with_capacity(new_length);

    for i in 0..new_length {
        new_array.push(array[indices[i]]);
    }

    new_array
}

pub fn compute_vertical_profile_theil_sen(
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
pub fn compute_quantile(quantile: f32, array: &Vec<f32>) -> f32 {
    let mut new_array: Vec<f32> = array.iter().copied().filter(|x| is_valid(*x)).collect();
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

    assert!(is_valid(exact_q));

    exact_q
}

pub fn invert_matrix(input: MatRef<'_, f32>, inverse: MatMut<'_, f32>) {
    let n = input.nrows();
    let mut lu = input.clone();
    // let mut lu = input.clone();
    let mut row_perm = vec![0, n];
    let mut row_perm_inv = vec![0, n];
    let (_, row_perm) = lu_in_place(lu., perm, perm_inv, parallelism, stack, params)
    
    // invert(dst, lu_factors, row_perm, parallelism, stack);

    todo!()
}
