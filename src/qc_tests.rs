use crate::{points::Points, util};

pub fn buddy_check(
    tree_points: Points,
    values: Vec<f32>,
    radii: Vec<f32>,
    nums_min: Vec<u32>,
    threshold: f32,
    max_elev_diff: f32,
    elev_gradient: f32,
    min_std: f32,
    num_iterations: u32,
    obs_to_check: Vec<bool>,
) -> Result<Vec<u32>, Box<dyn std::error::Error>> {
    // TODO: Check input vectors are properly sized

    let mut flags: Vec<u32> = values
        .iter()
        .map(|v| if util::is_valid(*v) { 0 } else { 1 })
        .collect();

    let check_all = obs_to_check.len() != values.len();

    let mut num_removed_last_iteration = 0;

    for _iteration in 1..=num_iterations {
        for i in 0..values.len() {
            let radius = if radii.len() == 1 { radii[0] } else { radii[i] };
            let num_min = if nums_min.len() == 1 {
                nums_min[0]
            } else {
                nums_min[i]
            };

            if flags[i] != 0 {
                continue;
            }

            if check_all || obs_to_check[i] {
                let (lat, lon, elev, _) = tree_points.get_coords_at_index(i);
                let neighbours = tree_points.get_neighbours(lat, lon, radius, false);

                let mut list_buddies: Vec<f32> = Vec::new();

                if neighbours.len() >= num_min as usize {
                    for neighbour in neighbours {
                        let (_, _, neighbour_elev, _) =
                            tree_points.get_coords_at_index(neighbour.data);

                        if flags[neighbour.data] != 0 {
                            continue;
                        }

                        if max_elev_diff > 0.0 {
                            let elev_diff = elev - neighbour_elev;

                            if elev_diff.abs() <= max_elev_diff {
                                let adjusted_value =
                                    values[neighbour.data] + (elev_diff * elev_gradient);

                                list_buddies.push(adjusted_value);
                            }
                        } else {
                            list_buddies.push(values[neighbour.data]);
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

                    if (values[i] - mean).abs() / std_adjusted > threshold {
                        flags[i] = 1;
                    }
                }
            }
        }

        let num_removed: u32 = flags.iter().sum();
        let num_removed_current_iteration = num_removed - num_removed_last_iteration;

        if num_removed_current_iteration == 0 {
            break;
        }

        num_removed_last_iteration = num_removed_current_iteration;
    }

    Ok(flags)
}
