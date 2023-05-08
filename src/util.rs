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
