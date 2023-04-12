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
