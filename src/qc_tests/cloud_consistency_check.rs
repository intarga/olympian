use crate::{Error, Flag};

// TODO: better documentation for this?
/// Consistency check between cloud parameters
pub fn cloud_consistency_check(
    // TODO: check these types are correct. Might actually be int?
    low_type_cloud: &[Option<f32>],
    medium_type_cloud: &[Option<f32>],
    cloud_base_height: &[Option<f32>],
) -> Result<Vec<Flag>, Error> {
    if low_type_cloud.len() != medium_type_cloud.len()
        || low_type_cloud.len() != cloud_base_height.len()
    {
        return Err(Error::InvalidInputShape(
            "all input slices must have the same dimensions".to_string(),
        ));
    }

    let windows = low_type_cloud
        .iter()
        .zip(medium_type_cloud)
        .zip(cloud_base_height);

    Ok(windows
        .map(|((low_type_cloud, medium_type_cloud), cloud_base_height)| {
            if low_type_cloud.is_none()
                || medium_type_cloud.is_none()
                || cloud_base_height.is_none()
            {
                Flag::DataMissing
            } else if cloud_base_height.unwrap() < 1500.
                && low_type_cloud.unwrap() == 0.
                && medium_type_cloud.unwrap() != 2.
            {
                Flag::Fail
            } else {
                Flag::Pass
            }
        })
        .collect())
}
