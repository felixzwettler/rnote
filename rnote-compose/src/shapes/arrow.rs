use kurbo::PathEl;
use na::Rotation2;
use p2d::bounding_volume::Aabb;
use serde::{Deserialize, Serialize};

use crate::helpers::{AabbHelpers, Vector2Helpers};
use crate::shapes::ShapeBehaviour;
use crate::transform::TransformBehaviour;

use super::Line;

type Radian = f64;

/// All doc-comments of this file rely on the following graphic:
/// ```
///         tip
///         /|\
///        / | \
///       /  |  \
///    lline |  rline
///          |
///          |
///          |
///         start
/// ```
/// Where `lline`, `tip`, `start` and `rline` represent a vector of the arrow.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default)]
#[serde(default, rename = "arrow")]
pub struct Arrow {
    /// The start of the arrow
    pub start: na::Vector2<f64>,

    /// The tip of the arow
    pub tip: na::Vector2<f64>,

    /// Metadata for `rline` and `lline`.
    tip_lines: TipLines,
}

impl TransformBehaviour for Arrow {
    fn translate(&mut self, offset: na::Vector2<f64>) {
        self.start += offset;
        self.tip += offset;
    }

    fn rotate(&mut self, angle: f64, center: na::Point2<f64>) {
        let isometry = {
            let mut isometry = na::Isometry2::identity();
            isometry.append_rotation_wrt_point_mut(&na::UnitComplex::new(angle), &center);
            isometry
        };

        self.start = (isometry * na::Point2::from(self.start)).coords;
        self.tip = (isometry * na::Point2::from(self.tip)).coords;
    }

    fn scale(&mut self, scale: na::Vector2<f64>) {
        self.start = self.start.component_mul(&scale);
        self.tip = self.tip.component_mul(&scale);
    }
}

impl ShapeBehaviour for Arrow {
    fn bounds(&self) -> Aabb {
        let (x_values, y_values) = {
            let lline = self.get_lline(None);
            let rline = self.get_rline(None);

            let x_values = [lline[0], rline[0], self.start[0], self.tip[0]];
            let y_values = [lline[1], rline[1], self.start[1], self.tip[1]];

            (x_values, y_values)
        };

        let bottom_left_corner = {
            let lowest_x = x_values.into_iter().reduce(f64::min).unwrap();
            let lowest_y = y_values.into_iter().reduce(f64::min).unwrap();
            na::Point2::new(lowest_x, lowest_y)
        };

        let top_right_corner = {
            let highest_x = x_values.into_iter().reduce(f64::max).unwrap();
            let highest_y = y_values.into_iter().reduce(f64::max).unwrap();
            na::Point2::new(highest_x, highest_y)
        };

        AabbHelpers::new_positive(bottom_left_corner, top_right_corner)
    }

    fn hitboxes(&self) -> Vec<Aabb> {
        let n_splits = super::hitbox_elems_for_shape_len((self.tip - self.start).norm());

        self.split(n_splits)
            .into_iter()
            .map(|line| line.bounds())
            .collect()
    }
}

impl Arrow {
    /// Creating a new arrow with the given start and tip vectors.
    pub fn new(start: na::Vector2<f64>, tip: na::Vector2<f64>) -> Self {
        Self {
            start,
            tip,
            ..Self::default()
        }
    }

    /// Splits itself given the no splits
    pub fn split(&self, n_splits: i32) -> Vec<Line> {
        (0..n_splits)
            .map(|i| {
                let sub_start = self
                    .start
                    .lerp(&self.tip, f64::from(i) / f64::from(n_splits));
                let sub_end = self
                    .start
                    .lerp(&self.tip, f64::from(i + 1) / f64::from(n_splits));

                Line {
                    start: sub_start,
                    end: sub_end,
                }
            })
            .collect::<Vec<Line>>()
    }

    /// convert the arrow to kurbo-elements.
    pub fn to_kurbo(&self) -> ArrowKurbo {
        let main = kurbo::Line::new(self.start.to_kurbo_point(), self.tip.to_kurbo_point());
        let tip_triangle = {
            let tip = self.tip.to_kurbo_point();
            let lline = self.get_lline().to_kurbo_point();
            let rline = self.get_rline().to_kurbo_point();

            kurbo::BezPath::from_vec(vec![
                PathEl::MoveTo(lline),
                PathEl::LineTo(tip),
                PathEl::LineTo(rline),
            ])
        };

        ArrowKurbo {
            stem: main,
            tip_triangle,
        }
    }
}

/// This implementation holds the functions to get the vectors `rline` and
/// `lline`.
impl Arrow {
    /// Computes and returns `rline`.
    /// Optional: `stroke_width` will be used to hearistically adjust the length
    /// of `lline`.
    pub fn get_lline(&self) -> na::Vector2<f64> {
        let vec_a = self.get_tip_stem();
        let rotation_matrix = self.get_rotation_matrix();

        rotation_matrix * vec_a + self.tip
    }

    /// Computes and returns `rline`
    /// Optional: `stroke_width` will be used to hearistically adjust the length
    /// of `lline`.
    pub fn get_rline(&self) -> na::Vector2<f64> {
        let vec_b = self.get_tip_stem();
        let rotation_matrix = self.get_rotation_matrix().transpose();

        rotation_matrix * vec_b + self.tip
    }

    /// Returns a direction vector from `start` to `tip` but with an
    /// appropriate length for `rline` and `lline` which then just
    /// need to be rotated.
    fn get_tip_stem(&self) -> na::Vector2<f64> {
        let stem_vector = self.get_norm_stem_vector();
        let length = TipLines::MIN_LENGTH
            .max(TipLines::MIN_LENGTH / 4.0 * self.tip_lines.length)
            .min(TipLines::MAX_LENGTH);

        stem_vector * length
    }

    /// Returns the (normalized) stem vector from `start` to `tip`.
    fn get_norm_stem_vector(&self) -> na::Vector2<f64> {
        let stem_vector = self.get_stem_vector();
        stem_vector / stem_vector.norm()
    }

    /// Returns the stem vector from `start` to `tip`.
    fn get_stem_vector(&self) -> na::Vector2<f64> {
        self.tip - self.start
    }

    fn get_rotation_matrix(&self) -> Rotation2<f64> {
        Rotation2::new(self.tip_lines.angle)
    }
}

/// A helper struct to store the metadata of `rline` and `lline`.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(default, rename = "arrow_tip_lines")]
struct TipLines {
    /// The angle of `rline` and `lline`.
    pub angle: Radian,

    /// The length of `rline` and `lline`.
    pub length: f64,
}

impl TipLines {
    /// The default angle for the `rline` and `lline`.
    const DEFAULT_ANGLE: Radian = (13.0 / 16.0) * std::f64::consts::PI;

    /// The min length for `rline` and `lline`.
    const MIN_LENGTH: f64 = 64.0;

    /// The max length for `rline` and `lline`.
    const MAX_LENGTH: f64 = 2.0 * Self::MIN_LENGTH;
}

impl Default for TipLines {
    fn default() -> Self {
        Self {
            angle: Self::DEFAULT_ANGLE,
            length: Self::MIN_LENGTH,
        }
    }
}

/// A helper struct which holds the kurbo-elements of the arrow.
#[derive(Debug, Clone, PartialEq)]
pub struct ArrowKurbo {
    /// This holds the line from `start` -> `tip`.
    pub stem: kurbo::Line,

    /// This holds the line from `lline` -> `tip` -> `rline`.
    pub tip_triangle: kurbo::BezPath,
}
