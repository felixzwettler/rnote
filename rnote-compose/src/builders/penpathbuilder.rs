use p2d::bounding_volume::{BoundingVolume, AABB};
use piet::RenderContext;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

use crate::penhelpers::PenEvent;
use crate::penpath::{Element, Segment};
use crate::shapes::CubicBezier;
use crate::style::Composer;
use crate::{PenPath, Shape, Style};

use super::shapebuilderbehaviour::{BuilderProgress, ShapeBuilderCreator};
use super::{Constraints, ShapeBuilderBehaviour};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) enum PenPathBuilderState {
    Start,
    During,
}

#[derive(Debug, Clone)]
/// The pen path builder
pub struct PenPathBuilder {
    pub(crate) state: PenPathBuilderState,
    /// Buffered elements, which are filled up by new pen events and used to try to build path segments
    pub buffer: VecDeque<Element>,
}

impl ShapeBuilderCreator for PenPathBuilder {
    fn start(element: Element) -> Self {
        let mut buffer = VecDeque::new();
        buffer.push_back(element);

        Self {
            state: PenPathBuilderState::Start,
            buffer,
        }
    }
}

impl ShapeBuilderBehaviour for PenPathBuilder {
    fn handle_event(&mut self, event: PenEvent, _constraint: Constraints) -> BuilderProgress {
        /*         log::debug!(
            "event: {:?}; buffer.len(): {}, state: {:?}",
            event,
            self.buffer.len(),
            self.state
        ); */

        match (&mut self.state, event) {
            (PenPathBuilderState::Start, PenEvent::Down { element, .. }) => {
                self.buffer.push_back(element);

                match self.try_build_segments_start() {
                    Some(shapes) => BuilderProgress::EmitContinue(shapes),
                    None => BuilderProgress::InProgress,
                }
            }
            (PenPathBuilderState::During, PenEvent::Down { element, .. }) => {
                self.buffer.push_back(element);

                match self.try_build_segments_during() {
                    Some(shapes) => BuilderProgress::EmitContinue(shapes),
                    None => BuilderProgress::InProgress,
                }
            }
            (_, PenEvent::Up { element, .. }) => {
                self.buffer.push_back(element);

                BuilderProgress::Finished(self.try_build_segments_end())
            }
            (_, PenEvent::Proximity { .. }) => BuilderProgress::InProgress,

            (_, PenEvent::KeyPressed { .. }) => BuilderProgress::InProgress,
            (_, PenEvent::Cancel) => {
                self.reset();

                BuilderProgress::Finished(vec![])
            }
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
        let penpath = match &self.state {
            PenPathBuilderState::Start => self
                .buffer
                .iter()
                .zip(self.buffer.iter().skip(1))
                .map(|(start, end)| Segment::Line {
                    start: *start,
                    end: *end,
                })
                .collect::<PenPath>(),
            // Skipping the first buffer element as that is the not drained by the segment builder and is the prev element in the "During" state
            PenPathBuilderState::During => self
                .buffer
                .iter()
                .skip(1)
                .zip(self.buffer.iter().skip(2))
                .map(|(start, end)| Segment::Line {
                    start: *start,
                    end: *end,
                })
                .collect::<PenPath>(),
        };

        penpath.draw_composed(cx, style);
        cx.restore().unwrap();
    }
}

impl PenPathBuilder {
    fn try_build_segments_start(&mut self) -> Option<Vec<Shape>> {
        match self.buffer.len() {
            3.. => {
                // Here we have enough elements to switch into during state
                self.state = PenPathBuilderState::During;

                Some(vec![Shape::Segment(Segment::Line {
                    start: self.buffer[0],
                    end: self.buffer[1],
                })])
            }
            _ => None,
        }
    }

    fn try_build_segments_during(&mut self) -> Option<Vec<Shape>> {
        match self.buffer.len() {
            4.. => {
                if let Some(cubbez) = CubicBezier::new_w_catmull_rom(
                    self.buffer[0].pos,
                    self.buffer[1].pos,
                    self.buffer[2].pos,
                    self.buffer[3].pos,
                ) {
                    let segment = Shape::Segment(Segment::CubBez {
                        start: Element {
                            pos: cubbez.start,
                            ..self.buffer[1]
                        },
                        cp1: cubbez.cp1,
                        cp2: cubbez.cp2,
                        end: Element {
                            pos: cubbez.end,
                            ..self.buffer[2]
                        },
                    });

                    self.buffer.pop_front();

                    Some(vec![segment])
                } else {
                    let segment = Shape::Segment(Segment::Line {
                        start: self.buffer[1],
                        end: self.buffer[2],
                    });

                    self.buffer.pop_front();

                    Some(vec![segment])
                }
            }
            _ => None,
        }
    }

    fn try_build_segments_end(&mut self) -> Vec<Shape> {
        let mut segments: Vec<Shape> = vec![];

        while let Some(mut new_segments) = match self.buffer.len() {
            0 => None,
            1 => Some(vec![Shape::Segment(Segment::Dot {
                element: self.buffer.remove(0).unwrap(),
            })]),
            2 => {
                let elements = self.buffer.drain(0..2).collect::<Vec<Element>>();
                Some(vec![Shape::Segment(Segment::Line {
                    start: elements[0],
                    end: elements[1],
                })])
            }
            3 => {
                let elements = self.buffer.drain(0..3).collect::<Vec<Element>>();
                Some(vec![Shape::Segment(Segment::Line {
                    start: elements[1],
                    end: elements[2],
                })])
            }
            4.. => {
                if let Some(cubbez) = CubicBezier::new_w_catmull_rom(
                    self.buffer[0].pos,
                    self.buffer[1].pos,
                    self.buffer[2].pos,
                    self.buffer[3].pos,
                ) {
                    let segment = Shape::Segment(Segment::CubBez {
                        start: Element {
                            pos: cubbez.start,
                            ..self.buffer[1]
                        },
                        cp1: cubbez.cp1,
                        cp2: cubbez.cp2,
                        end: Element {
                            pos: cubbez.end,
                            ..self.buffer[2]
                        },
                    });

                    // Only remove one element as more segments can be built
                    self.buffer.pop_front();

                    Some(vec![segment])
                } else {
                    let segment = Shape::Segment(Segment::Line {
                        start: self.buffer[1],
                        end: self.buffer[2],
                    });

                    self.buffer.pop_front();

                    Some(vec![segment])
                }
            }
            _ => None,
        } {
            segments.append(&mut new_segments);
        }

        self.reset();

        segments
    }

    fn reset(&mut self) {
        self.buffer.clear();
        self.state = PenPathBuilderState::Start;
    }
}
