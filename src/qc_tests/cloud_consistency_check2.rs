use crate::{Error, Flag};

// TODO: better name for this?
// TODO: better documentation for this?
/// Consistency check between cloud parameters
pub fn cloud_consistency_check2(
    // TODO: check these types are correct. Might actually be int?
    // TODO: avoid name shadowing?
    low_type_cloud: &[Option<f32>],
    cloud_area_fraction: &[Option<f32>],
    cloud_base_height: &[Option<f32>],
) -> Result<Vec<Flag>, Error> {
    if low_type_cloud.len() != cloud_area_fraction.len()
        || low_type_cloud.len() != cloud_base_height.len()
    {
        return Err(Error::InvalidInputShape(
            "all input slices must have the same dimensions".to_string(),
        ));
    }

    let windows = low_type_cloud
        .iter()
        .zip(cloud_area_fraction)
        .zip(cloud_base_height);

    Ok(windows
        .map(
            |((low_type_cloud, cloud_area_fraction), cloud_base_height)| {
                if low_type_cloud.is_none()
                    || cloud_area_fraction.is_none()
                    || cloud_base_height.is_none()
                {
                    Flag::DataMissing
                } else if cloud_base_height.unwrap() < 2500.
                    && low_type_cloud.unwrap() != 0.
                    && cloud_area_fraction.unwrap() != 0.
                {
                    Flag::Fail
                } else {
                    Flag::Pass
                }
            },
        )
        .collect())
}
