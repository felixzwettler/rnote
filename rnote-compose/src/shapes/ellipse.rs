use p2d::bounding_volume::AABB;
use serde::{Deserialize, Serialize};

use crate::shapes::ShapeBehaviour;
use crate::transform::TransformBehaviour;
use crate::Transform;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, rename = "ellipse")]
/// A Ellipse
pub struct Ellipse {
    /// The radii of the ellipse
    #[serde(rename = "radii")]
    pub radii: na::Vector2<f64>,
    /// The transform
    #[serde(rename = "transform")]
    pub transform: Transform,
}

impl Default for Ellipse {
    fn default() -> Self {
        Self {
            radii: na::Vector2::zeros(),
            transform: Transform::default(),
        }
    }
}

impl TransformBehaviour for Ellipse {
    fn translate(&mut self, offset: nalgebra::Vector2<f64>) {
        self.transform.append_translation_mut(offset);
    }

    fn rotate(&mut self, angle: f64, center: nalgebra::Point2<f64>) {
        self.transform.append_rotation_wrt_point_mut(angle, center)
    }

    fn scale(&mut self, scale: na::Vector2<f64>) {
        self.transform.append_scale_mut(scale);
    }
}

impl ShapeBehaviour for Ellipse {
    fn bounds(&self) -> AABB {
        let center = self.transform.affine * na::point![0.0, 0.0];
        // using a vector to ignore the translation
        let half_extents = na::Vector2::from_homogeneous(
            self.transform.affine.into_inner().abs() * self.radii.to_homogeneous(),
        )
        .unwrap();

        AABB::from_half_extents(center, half_extents)
    }
}
