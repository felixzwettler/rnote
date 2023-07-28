// Imports
use super::shapebuildable::{ShapeBuilderCreator, ShapeBuilderProgress};
use super::ShapeBuildable;
use crate::constraints::ConstraintRatio;
use crate::ext::AabbExt;
use crate::penevents::{PenEvent, PenState};
use crate::penpath::Element;
use crate::shapes::Ellipse;
use crate::style::{indicators, Composer};
use crate::Constraints;
use crate::{Shape, Style};
use p2d::bounding_volume::{Aabb, BoundingVolume};
use piet::RenderContext;
use std::time::Instant;

#[derive(Debug, Clone)]
/// Foci ellipse builder state.
pub enum FociEllipseBuilderState {
    Start(na::Vector2<f64>),
    StartFinished(na::Vector2<f64>),
    Foci([na::Vector2<f64>; 2]),
    FociFinished([na::Vector2<f64>; 2]),
    FociAndPoint {
        foci: [na::Vector2<f64>; 2],
        point: na::Vector2<f64>,
    },
}

#[derive(Debug, Clone)]
/// Ellipse builder with foci and point.
pub struct FociEllipseBuilder {
    state: FociEllipseBuilderState,
}

impl ShapeBuilderCreator for FociEllipseBuilder {
    fn start(element: Element, _now: Instant) -> Self {
        Self {
            state: FociEllipseBuilderState::Start(element.pos),
        }
    }
}

impl ShapeBuildable for FociEllipseBuilder {
    fn handle_event(
        &mut self,
        event: PenEvent,
        _now: Instant,
        mut constraints: Constraints,
    ) -> ShapeBuilderProgress {
        match (&mut self.state, event) {
            (FociEllipseBuilderState::Start(first), PenEvent::Down { element, .. }) => {
                *first = element.pos;
            }
            (FociEllipseBuilderState::Start(_), PenEvent::Up { element, .. }) => {
                self.state = FociEllipseBuilderState::StartFinished(element.pos);
            }
            (FociEllipseBuilderState::Start(_), _) => {}
            (FociEllipseBuilderState::StartFinished(first), PenEvent::Down { element, .. }) => {
                // we want to allow horizontal and vertical constraints while setting the second foci
                constraints.ratios.insert(ConstraintRatio::Horizontal);
                constraints.ratios.insert(ConstraintRatio::Vertical);

                self.state = FociEllipseBuilderState::Foci([
                    *first,
                    constraints.constrain(element.pos - *first) + *first,
                ]);
            }
            (FociEllipseBuilderState::StartFinished(_), _) => {}
            (FociEllipseBuilderState::Foci(foci), PenEvent::Down { element, .. }) => {
                constraints.ratios.insert(ConstraintRatio::Horizontal);
                constraints.ratios.insert(ConstraintRatio::Vertical);

                foci[1] = constraints.constrain(element.pos - foci[0]) + foci[0];
            }
            (FociEllipseBuilderState::Foci(foci), PenEvent::Up { element, .. }) => {
                constraints.ratios.insert(ConstraintRatio::Horizontal);
                constraints.ratios.insert(ConstraintRatio::Vertical);

                self.state = FociEllipseBuilderState::FociFinished([
                    foci[0],
                    constraints.constrain(element.pos - foci[0]) + foci[0],
                ]);
            }
            (FociEllipseBuilderState::Foci(_), _) => {}
            (FociEllipseBuilderState::FociFinished(foci), PenEvent::Down { element, .. }) => {
                constraints.ratios.insert(ConstraintRatio::Horizontal);
                constraints.ratios.insert(ConstraintRatio::Vertical);

                self.state = FociEllipseBuilderState::FociAndPoint {
                    foci: *foci,
                    point: constraints.constrain(element.pos - foci[1]) + foci[1],
                };
            }
            (FociEllipseBuilderState::FociFinished(_), _) => {}
            (
                FociEllipseBuilderState::FociAndPoint { foci: _, point },
                PenEvent::Down { element, .. },
            ) => {
                *point = element.pos;
            }
            (FociEllipseBuilderState::FociAndPoint { foci, point }, PenEvent::Up { .. }) => {
                let shape = Ellipse::from_foci_and_point(*foci, *point);

                return ShapeBuilderProgress::Finished(vec![Shape::Ellipse(shape)]);
            }
            (FociEllipseBuilderState::FociAndPoint { .. }, _) => {}
        }

        ShapeBuilderProgress::InProgress
    }

    fn bounds(&self, style: &Style, zoom: f64) -> Option<Aabb> {
        let stroke_width = style.stroke_width();

        match &self.state {
            FociEllipseBuilderState::Start(first)
            | FociEllipseBuilderState::StartFinished(first) => Some(Aabb::from_half_extents(
                (*first).into(),
                na::Vector2::repeat(stroke_width.max(indicators::POS_INDICATOR_RADIUS) / zoom),
            )),
            FociEllipseBuilderState::Foci(foci) | FociEllipseBuilderState::FociFinished(foci) => {
                Some(
                    Aabb::new_positive(foci[0].into(), foci[1].into())
                        .loosened(stroke_width.max(indicators::POS_INDICATOR_RADIUS) / zoom),
                )
            }
            FociEllipseBuilderState::FociAndPoint { foci, point } => {
                let ellipse = Ellipse::from_foci_and_point(*foci, *point);

                Some(
                    ellipse
                        .composed_bounds(style)
                        .loosened(indicators::POS_INDICATOR_RADIUS / zoom),
                )
            }
        }
    }

    fn draw_styled(&self, cx: &mut piet_cairo::CairoRenderContext, style: &Style, zoom: f64) {
        cx.save().unwrap();
        match &self.state {
            FociEllipseBuilderState::Start(first)
            | FociEllipseBuilderState::StartFinished(first) => {
                indicators::draw_pos_indicator(cx, PenState::Down, *first, zoom);
            }
            FociEllipseBuilderState::Foci(foci) | FociEllipseBuilderState::FociFinished(foci) => {
                indicators::draw_pos_indicator(cx, PenState::Up, foci[0], zoom);
                indicators::draw_pos_indicator(cx, PenState::Down, foci[1], zoom);
            }
            FociEllipseBuilderState::FociAndPoint { foci, point } => {
                let ellipse = Ellipse::from_foci_and_point(*foci, *point);
                ellipse.draw_composed(cx, style);

                indicators::draw_vec_indicator(cx, PenState::Down, foci[0], *point, zoom);
                indicators::draw_vec_indicator(cx, PenState::Down, foci[1], *point, zoom);
                indicators::draw_pos_indicator(cx, PenState::Up, foci[0], zoom);
                indicators::draw_pos_indicator(cx, PenState::Up, foci[1], zoom);
                indicators::draw_pos_indicator(cx, PenState::Down, *point, zoom);
            }
        }
        cx.restore().unwrap();
    }
}
