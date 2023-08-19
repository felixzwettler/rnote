use p2d::bounding_volume::{Aabb, BoundingVolume};
use piet::RenderContext;
use std::time::Instant;

use super::shapebuildable::{ShapeBuilderCreator, ShapeBuilderProgress};
use super::ShapeBuildable;
use crate::penevent::{PenEvent, PenState};
use crate::penpath::Element;
use crate::shapes::Line;
use crate::style::{indicators, Composer};
use crate::Constraints;
use crate::{Shape, Style};

/// 2D single quadrant coordinate system builder.
#[derive(Debug, Clone)]
pub struct QuadrantCoordSystem2DBuilder {
    /// Tip of the y axis.
    tip_y: na::Vector2<f64>,
    /// Tip of the x axis.
    tip_x: na::Vector2<f64>,
}

impl ShapeBuilderCreator for QuadrantCoordSystem2DBuilder {
    fn start(element: Element, _now: Instant) -> Self {
        Self {
            tip_y: element.pos,
            tip_x: element.pos,
        }
    }
}

impl ShapeBuildable for QuadrantCoordSystem2DBuilder {
    fn handle_event(
        &mut self,
        event: PenEvent,
        _now: Instant,
        constraints: Constraints,
    ) -> ShapeBuilderProgress {
        match event {
            PenEvent::Down { element, .. } => {
                self.tip_x = constraints.constrain(element.pos - self.tip_y) + self.tip_y;
            }
            PenEvent::Up { .. } => {
                return ShapeBuilderProgress::Finished(
                    self.state_as_lines()
                        .iter()
                        .map(|&line| Shape::Line(line))
                        .collect::<Vec<Shape>>(),
                );
            }
            _ => {}
        }

        ShapeBuilderProgress::InProgress
    }

    fn bounds(&self, style: &Style, zoom: f64) -> Option<Aabb> {
        Some(
            self.state_as_lines()
                .iter()
                .map(|line| line.composed_bounds(style))
                .fold(Aabb::new_invalid(), |acc, x| acc.merged(&x))
                .loosened(indicators::POS_INDICATOR_RADIUS / zoom),
        )
    }

    fn draw_styled(&self, cx: &mut piet_cairo::CairoRenderContext, style: &Style, zoom: f64) {
        cx.save().unwrap();

        for line in self.state_as_lines() {
            line.draw_composed(cx, style);
        }

        indicators::draw_pos_indicator(cx, PenState::Up, self.tip_y, zoom);
        indicators::draw_pos_indicator(cx, PenState::Down, self.tip_x, zoom);
        cx.restore().unwrap();
    }
}

impl QuadrantCoordSystem2DBuilder {
    /// The current state as two individual lines.
    pub fn state_as_lines(&self) -> Vec<Line> {
        let center = na::vector!(self.tip_y.x, self.tip_x.y);

        let up_axis = Line {
            start: center,
            end: self.tip_y,
        };

        let right_axis = Line {
            start: center,
            end: self.tip_x,
        };

        vec![up_axis, right_axis]
    }
}
