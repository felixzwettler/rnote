// Imports
use super::shapebuildable::{ShapeBuilderCreator, ShapeBuilderProgress};
use super::ShapeBuildable;
use crate::penevents::{PenEvent, PenState};
use crate::penpath::Element;
use crate::shapes::Rectangle;
use crate::style::{indicators, Composer};
use crate::Constraints;
use crate::{Shape, Style, Transform};
use p2d::bounding_volume::{Aabb, BoundingVolume};
use p2d::shape::Cuboid;
use piet::RenderContext;
use std::time::Instant;

/// Rectangle builder.
#[derive(Debug, Clone)]
pub struct RectangleBuilder {
    /// Start position.
    start: na::Vector2<f64>,
    /// Current position.
    current: na::Vector2<f64>,
}

impl ShapeBuilderCreator for RectangleBuilder {
    fn start(element: Element, _now: Instant) -> Self {
        Self {
            start: element.pos,
            current: element.pos,
        }
    }
}

impl ShapeBuildable for RectangleBuilder {
    fn handle_event(
        &mut self,
        event: PenEvent,
        _now: Instant,
        constraints: Constraints,
    ) -> ShapeBuilderProgress {
        match event {
            PenEvent::Down { element, .. } => {
                self.current = constraints.constrain(element.pos - self.start) + self.start;
            }
            PenEvent::Up { .. } => {
                return ShapeBuilderProgress::Finished(vec![Shape::Rectangle(
                    self.state_as_rect(),
                )]);
            }
            _ => {}
        }

        ShapeBuilderProgress::InProgress
    }

    fn bounds(&self, style: &Style, zoom: f64) -> Option<Aabb> {
        Some(
            self.state_as_rect()
                .composed_bounds(style)
                .loosened(indicators::POS_INDICATOR_RADIUS / zoom),
        )
    }

    fn draw_styled(&self, cx: &mut piet_cairo::CairoRenderContext, style: &Style, zoom: f64) {
        cx.save().unwrap();
        let rect = self.state_as_rect();
        rect.draw_composed(cx, style);

        indicators::draw_pos_indicator(cx, PenState::Up, self.start, zoom);
        indicators::draw_pos_indicator(cx, PenState::Down, self.current, zoom);
        cx.restore().unwrap();
    }
}

impl RectangleBuilder {
    /// The current state as a rectangle.
    pub fn state_as_rect(&self) -> Rectangle {
        let center = (self.start + self.current) * 0.5;
        let transform = Transform::new_w_isometry(na::Isometry2::new(center, 0.0));
        let half_extents = (self.current - self.start) * 0.5;
        let cuboid = Cuboid::new(half_extents);

        Rectangle { cuboid, transform }
    }
}
