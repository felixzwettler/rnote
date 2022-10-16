use ink_stroke_modeler_rs::{
    ModelerInput, ModelerInputEventType, StrokeModeler, StrokeModelerParams,
};
use p2d::bounding_volume::{BoundingVolume, AABB};
use piet::RenderContext;
use std::collections::VecDeque;
use std::time::Instant;

use crate::penhelpers::PenEvent;
use crate::penpath::{Element, Segment};
use crate::style::Composer;
use crate::{PenPath, Shape, Style};

use super::shapebuilderbehaviour::{BuilderProgress, ShapeBuilderCreator};
use super::{Constraints, ShapeBuilderBehaviour};

/// The pen path builder
pub struct PenPathModeledBuilder {
    /// Buffered elements, which are filled up by new pen events and used to try to build path segments
    pub buffer: VecDeque<Element>,
    start_time: Instant,
    last_element: Element,
    last_element_time: Instant,
    stroke_modeler: StrokeModeler,
}

impl std::fmt::Debug for PenPathModeledBuilder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ModeledPenPathBuilder")
            .field("buffer", &self.buffer)
            .field("start_time", &self.start_time)
            .field("last_element", &self.last_element)
            .field("last_element_time", &self.last_element_time)
            .field("stroke_modeler", &".. no debug impl ..")
            .finish()
    }
}

impl ShapeBuilderCreator for PenPathModeledBuilder {
    fn start(element: Element, now: Instant) -> Self {
        let now = Instant::now();
        let buffer = VecDeque::new();

        let stroke_modeler = StrokeModeler::default();

        let mut builder = Self {
            buffer,
            start_time: now,
            last_element: element,
            last_element_time: now,
            stroke_modeler,
        };

        builder.restart(element, now);

        builder
    }
}

impl ShapeBuilderBehaviour for PenPathModeledBuilder {
    fn handle_event(
        &mut self,
        event: PenEvent,
        _now: Instant,
        _constraints: Constraints,
    ) -> BuilderProgress {
        /*         log::debug!(
            "event: {:?}; buffer.len(): {}, state: {:?}",
            event,
            self.buffer.len(),
            self.state
        ); */

        match event {
            PenEvent::Down { element, .. } => {
                // kDown is already fed when instanciating the builder
                self.update_modeler_w_element(element, ModelerInputEventType::kMove);

                match self.try_build_segments() {
                    Some(shapes) => BuilderProgress::EmitContinue(shapes),
                    None => BuilderProgress::InProgress,
                }
            }
            PenEvent::Up { element, .. } => {
                self.update_modeler_w_element(element, ModelerInputEventType::kUp);

                let segment = self.try_build_segments().unwrap_or_else(|| vec![]);

                BuilderProgress::Finished(segment)
            }
            PenEvent::Proximity { .. } | PenEvent::KeyPressed { .. } | PenEvent::Text { .. } => {
                BuilderProgress::InProgress
            }
            PenEvent::Cancel => BuilderProgress::Finished(vec![]),
        }
    }

    fn bounds(&self, style: &Style, zoom: f64) -> Option<AABB> {
        let stroke_width = style.stroke_width();

        if self.buffer.is_empty() {
            return None;
        }

        Some(self.buffer.iter().fold(AABB::new_invalid(), |mut acc, x| {
            acc.take_point(na::Point2::from(x.pos));
            acc.loosened(stroke_width / zoom)
        }))
    }

    fn draw_styled(&self, cx: &mut piet_cairo::CairoRenderContext, style: &Style, _zoom: f64) {
        cx.save().unwrap();

        let prediction = self
            .stroke_modeler
            .predict()
            .into_iter()
            .map(|r| {
                let pos = r.get_pos();
                let pressure = r.get_pressure();

                Element::new(na::vector![pos.0 as f64, pos.1 as f64], pressure as f64)
            })
            .collect::<Vec<Element>>();

        let penpath = prediction
            .iter()
            .zip(prediction.iter().skip(1))
            .map(|(start, end)| Segment::Line {
                start: *start,
                end: *end,
            })
            .collect::<PenPath>();

        /*
               // Change prediction stroke color for debugging
               let mut style = style.clone();
               match style {
                   Style::Smooth(ref mut smooth_options) => {
                       smooth_options.stroke_color = Some(crate::Color::RED)
                   }
                   _ => {}
               }
        */

        penpath.draw_composed(cx, style);
        cx.restore().unwrap();
    }
}

impl PenPathModeledBuilder {
    const MODELER_MIN_OUTPUT_RATE: f64 = 180.0;
    const MODELER_MAX_OUTPUTS_PER_CALL: i32 = 100;

    fn try_build_segments(&mut self) -> Option<Vec<Shape>> {
        if self.buffer.len() < 2 {
            return None;
        }
        let mut segments = vec![];

        while self.buffer.len() > 2 {
            segments.push(Shape::Segment(Segment::Line {
                start: self.buffer[0],
                end: self.buffer[1],
            }));

            self.buffer.pop_front();
        }

        Some(segments)
    }

    fn update_modeler_w_element(&mut self, element: Element, event_type: ModelerInputEventType) {
        let now = Instant::now();

        if self.last_element == element {
            // Can't feed modeler with duplicate elements, or results in `INVALID_ARGUMENT` errors
            return;
        }
        self.last_element = element;

        let n_steps = (now.duration_since(self.last_element_time).as_secs_f64()
            * Self::MODELER_MIN_OUTPUT_RATE)
            .ceil() as i32;

        if n_steps > Self::MODELER_MAX_OUTPUTS_PER_CALL {
            // If the no of outputs the modeler would need to produce exceeds the configured maximum
            // ( because the time delta between the last elements is too large ), it needs to be restarted.
            log::info!("penpathmodeledbuilder: update_modeler_w_element(): n_steps exceeds configured max outputs per call, restarting modeler");

            self.restart(element, now);
        }
        self.last_element_time = now;

        let modeler_input = ModelerInput::new(
            event_type,
            (element.pos[0] as f32, element.pos[1] as f32),
            now.duration_since(self.start_time).as_secs_f64(),
            element.pressure as f32,
            0.0,
            0.0,
        );

        //log::debug!("{modeler_input}");

        self.buffer.extend(
            self.stroke_modeler
                .update(modeler_input)
                .into_iter()
                .map(|r| {
                    let pos = r.get_pos();
                    let pressure = r.get_pressure();

                    Element::new(na::vector![pos.0 as f64, pos.1 as f64], pressure as f64)
                }),
        );
    }

    fn restart(&mut self, element: Element, now: Instant) {
        let params = StrokeModelerParams {
            sampling_min_output_rate: Self::MODELER_MIN_OUTPUT_RATE,
            sampling_max_outputs_per_call: Self::MODELER_MAX_OUTPUTS_PER_CALL,
            ..Default::default()
        };

        self.buffer.clear();
        self.start_time = now;
        self.last_element_time = now;
        self.last_element = element;
        self.stroke_modeler.reset(params);

        self.buffer.extend(
            self.stroke_modeler
                .update(ModelerInput::new(
                    ModelerInputEventType::kDown,
                    (element.pos[0] as f32, element.pos[1] as f32),
                    0.0,
                    element.pressure as f32,
                    0.0,
                    0.0,
                ))
                .into_iter()
                .map(|r| {
                    let pos = r.get_pos();
                    let pressure = r.get_pressure();

                    Element::new(na::vector![pos.0 as f64, pos.1 as f64], pressure as f64)
                }),
        );
    }
}
