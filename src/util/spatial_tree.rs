use crate::util;
use rstar::{primitives::GeomWithData, RTree};

/// A point in the [`SpatialTree`]
///
/// The `[f32; 3]` represents the xyz coordinates of the point, which is used
/// to spatially index, and the usize represents the index into the lats, lons,
/// elevs, and values arrays associated with that point.
pub(crate) type SpatialPoint = GeomWithData<[f32; 3], usize>;

/// An R-tree to spatially index data to spatially index data
///
/// This allows a data point's nearest neighbours to be found with ease
#[derive(Debug, Clone)]
pub struct SpatialTree {
    pub(crate) tree: RTree<SpatialPoint>,
    pub lats: Vec<f32>,
    pub lons: Vec<f32>,
    pub elevs: Vec<f32>,
}

impl SpatialTree {
    /// Construct a SpatialTree from a set of positions
    ///
    /// The positions are specified by vectors of lats, lons, and elevs, where
    /// the elements from each vector at a given index together specify a
    /// single point in space
    pub fn from_latlons(lats: Vec<f32>, lons: Vec<f32>, elevs: Vec<f32>) -> Self {
        //TODO: ensure vecs are the same size

        let raw_points: Vec<SpatialPoint> = lats
            .iter()
            .zip(lons.iter())
            .enumerate()
            .map(|(i, (lat, lon))| {
                let (x, y, z) = util::convert_coordinates(*lat, *lon);
                SpatialPoint::new([x, y, z], i)
            })
            .collect();

        let tree = RTree::bulk_load(raw_points);

        Self {
            tree,
            lats,
            lons,
            elevs,
        }
    }

    pub(crate) fn get_neighbours(
        &self,
        lat: f32,
        lon: f32,
        radius: f32,
        include_match: bool,
    ) -> Vec<&SpatialPoint> {
        let (x, y, z) = util::convert_coordinates(lat, lon);

        let match_iter = self.tree.locate_within_distance([x, y, z], radius);

        match include_match {
            true => match_iter.collect(),
            false => match_iter
                .filter(|point| *point.geom() != [x, y, z])
                .collect(),
        }
    }

    pub(crate) fn get_neighbours_with_distance(
        &self,
        lat: f32,
        lon: f32,
        radius: f32,
        include_match: bool,
    ) -> (Vec<&SpatialPoint>, Vec<f32>) {
        let points = self.get_neighbours(lat, lon, radius, include_match);
        let vec_length = points.len();

        let mut distances = vec![0.; vec_length];

        let (x, y, z) = util::convert_coordinates(lat, lon);

        for i in 0..vec_length {
            let (x1, y1, z1) =
                util::convert_coordinates(self.lats[points[i].data], self.lons[points[i].data]);
            distances[i] = util::calc_distance_xyz(x, y, z, x1, y1, z1)
        }

        (points, distances)
    }

    pub(crate) fn get_coords_at_index(&self, i: usize) -> (f32, f32, f32) {
        (self.lats[i], self.lons[i], self.elevs[i])
    }
}
