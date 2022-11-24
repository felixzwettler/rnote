use crate::engine::{EngineView, EngineViewMut};
use crate::store::StrokeKey;
use crate::{DrawOnDocBehaviour, WidgetFlags};
use piet::RenderContext;
use rnote_compose::color;
use rnote_compose::helpers::{AABBHelpers, Vector2Helpers};
use rnote_compose::penhelpers::PenEvent;

use p2d::bounding_volume::AABB;
use serde::{Deserialize, Serialize};

use super::penbehaviour::{PenBehaviour, PenProgress};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default, rename = "verticalspace_tool")]
pub struct VerticalSpaceTool {
    #[serde(skip)]
    start_pos_y: f64,
    #[serde(skip)]
    current_pos_y: f64,
    #[serde(skip)]
    strokes_below: Vec<StrokeKey>,
}

impl Default for VerticalSpaceTool {
    fn default() -> Self {
        Self {
            start_pos_y: 0.0,
            current_pos_y: 0.0,
            strokes_below: vec![],
        }
    }
}

impl VerticalSpaceTool {
    const Y_OFFSET_THRESHOLD: f64 = 0.1;

    const FILL_COLOR: piet::Color = color::GNOME_BRIGHTS[2].with_a8(0x17);
    const THRESHOLD_LINE_COLOR: piet::Color = color::GNOME_GREENS[4].with_a8(0xf0);
    const OFFSET_LINE_COLOR: piet::Color = color::GNOME_BLUES[3];

    const THRESHOLD_LINE_WIDTH: f64 = 4.0;
    const OFFSET_LINE_WIDTH: f64 = 2.0;
}

impl DrawOnDocBehaviour for VerticalSpaceTool {
    fn bounds_on_doc(&self, engine_view: &EngineView) -> Option<AABB> {
        let viewport = engine_view.camera.viewport();

        let x = viewport.mins[0];
        let y = self.start_pos_y;
        let width = viewport.extents()[0];
        let height = self.current_pos_y - self.start_pos_y;
        let tool_bounds = AABB::new_positive(na::point![x, y], na::point![x + width, y + height]);

        Some(tool_bounds)
    }

    fn draw_on_doc(
        &self,
        cx: &mut piet_cairo::CairoRenderContext,
        engine_view: &EngineView,
    ) -> anyhow::Result<()> {
        cx.save().map_err(|e| anyhow::anyhow!("{}", e))?;

        let viewport = engine_view.camera.viewport();
        let x = viewport.mins[0];
        let y = self.start_pos_y;
        let width = viewport.extents()[0];
        let height = self.current_pos_y - self.start_pos_y;
        let tool_bounds = AABB::new_positive(na::point![x, y], na::point![x + width, y + height]);

        let tool_bounds_rect = kurbo::Rect::from_points(
            tool_bounds.mins.coords.to_kurbo_point(),
            tool_bounds.maxs.coords.to_kurbo_point(),
        );
        cx.fill(tool_bounds_rect, &Self::FILL_COLOR);

        let threshold_line =
            kurbo::Line::new(kurbo::Point::new(x, y), kurbo::Point::new(x + width, y));

        cx.stroke_styled(
            threshold_line,
            &Self::THRESHOLD_LINE_COLOR,
            Self::THRESHOLD_LINE_WIDTH,
            &piet::StrokeStyle::new().dash_pattern(&[12.0, 6.0]),
        );

        let offset_line = kurbo::Line::new(
            kurbo::Point::new(x, y + height),
            kurbo::Point::new(x + width, y + height),
        );
        cx.stroke(
            offset_line,
            &Self::OFFSET_LINE_COLOR,
            Self::OFFSET_LINE_WIDTH,
        );

        cx.restore().map_err(|e| anyhow::anyhow!("{}", e))?;
        Ok(())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default, rename = "offsetcamera_tool")]
pub struct OffsetCameraTool {
    #[serde(skip)]
    pub start: na::Vector2<f64>,
}

impl Default for OffsetCameraTool {
    fn default() -> Self {
        Self {
            start: na::Vector2::zeros(),
        }
    }
}

impl OffsetCameraTool {
    const DRAW_SIZE: na::Vector2<f64> = na::vector![16.0, 16.0];
    const FILL_COLOR: piet::Color = color::GNOME_DARKS[3].with_a8(0xf0);
    const OUTLINE_COLOR: piet::Color = color::GNOME_BRIGHTS[1].with_a8(0xf0);
    const PATH_WIDTH: f64 = 2.0;

    const CURSOR_PATH: &str = "m 8 1.078125 l -3 3 h 2 v 2.929687 h -2.960938 v -2 l -3 3 l 3 3 v -2 h 2.960938 v 2.960938 h -2 l 3 3 l 3 -3 h -2 v -2.960938 h 3.054688 v 2 l 3 -3 l -3 -3 v 2 h -3.054688 v -2.929687 h 2 z m 0 0";
}

impl DrawOnDocBehaviour for OffsetCameraTool {
    fn bounds_on_doc(&self, engine_view: &EngineView) -> Option<AABB> {
        Some(AABB::from_half_extents(
            na::Point2::from(self.start),
            ((Self::DRAW_SIZE + na::Vector2::repeat(Self::PATH_WIDTH)) * 0.5)
                / engine_view.camera.total_zoom(),
        ))
    }

    fn draw_on_doc(
        &self,
        cx: &mut piet_cairo::CairoRenderContext,
        engine_view: &EngineView,
    ) -> anyhow::Result<()> {
        cx.save().map_err(|e| anyhow::anyhow!("{}", e))?;

        if let Some(bounds) = self.bounds_on_doc(engine_view) {
            cx.transform(kurbo::Affine::translate(bounds.mins.coords.to_kurbo_vec()));
            cx.transform(kurbo::Affine::scale(1.0 / engine_view.camera.total_zoom()));

            let bez_path = kurbo::BezPath::from_svg(Self::CURSOR_PATH).unwrap();

            cx.stroke(bez_path.clone(), &Self::OUTLINE_COLOR, Self::PATH_WIDTH);
            cx.fill(bez_path, &Self::FILL_COLOR);
        }

        cx.restore().map_err(|e| anyhow::anyhow!("{}", e))?;
        Ok(())
    }
}

#[derive(
    Debug,
    Clone,
    Copy,
    Serialize,
    Deserialize,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    num_derive::FromPrimitive,
    num_derive::ToPrimitive,
)]
#[serde(rename = "tools_style")]
pub enum ToolsStyle {
    #[serde(rename = "verticalspace")]
    VerticalSpace,
    #[serde(rename = "offsetcamera")]
    OffsetCamera,
}

impl Default for ToolsStyle {
    fn default() -> Self {
        Self::VerticalSpace
    }
}

impl TryFrom<u32> for ToolsStyle {
    type Error = anyhow::Error;

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        num_traits::FromPrimitive::from_u32(value).ok_or_else(|| {
            anyhow::anyhow!("ToolsStyle try_from::<u32>() for value {} failed", value)
        })
    }
}

#[derive(Debug, Clone, Copy)]
enum ToolsState {
    Idle,
    Active,
}

impl Default for ToolsState {
    fn default() -> Self {
        Self::Idle
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default, rename = "tools")]
pub struct Tools {
    #[serde(rename = "style")]
    pub style: ToolsStyle,
    #[serde(rename = "verticalspace_tool")]
    pub verticalspace_tool: VerticalSpaceTool,
    #[serde(rename = "offsetcamera_tool")]
    pub offsetcamera_tool: OffsetCameraTool,

    #[serde(skip)]
    state: ToolsState,
}

impl PenBehaviour for Tools {
    fn handle_event(
        &mut self,
        event: PenEvent,
        engine_view: &mut EngineViewMut,
    ) -> (PenProgress, WidgetFlags) {
        let mut widget_flags = WidgetFlags::default();

        let pen_progress = match (&mut self.state, event) {
            (
                ToolsState::Idle,
                PenEvent::Down {
                    element,
                    shortcut_keys: _,
                },
            ) => {
                widget_flags.merge_with_other(engine_view.store.record());

                match self.style {
                    ToolsStyle::VerticalSpace => {
                        self.verticalspace_tool.start_pos_y = element.pos[1];
                        self.verticalspace_tool.current_pos_y = element.pos[1];

                        self.verticalspace_tool.strokes_below = engine_view
                            .store
                            .keys_below_y_pos(self.verticalspace_tool.current_pos_y);
                    }
                    ToolsStyle::OffsetCamera => {
                        self.offsetcamera_tool.start = element.pos;
                    }
                }

                self.state = ToolsState::Active;

                engine_view
                    .doc
                    .resize_autoexpand(engine_view.store, engine_view.camera);

                widget_flags.redraw = true;
                widget_flags.resize = true;
                widget_flags.indicate_changed_store = true;
                widget_flags.hide_scrollbars = Some(true);

                PenProgress::InProgress
            }
            (ToolsState::Idle, _) => PenProgress::Idle,
            (
                ToolsState::Active,
                PenEvent::Down {
                    element,
                    shortcut_keys: _,
                },
            ) => {
                let pen_progress = match self.style {
                    ToolsStyle::VerticalSpace => {
                        let y_offset = element.pos[1] - self.verticalspace_tool.current_pos_y;

                        if y_offset.abs() > VerticalSpaceTool::Y_OFFSET_THRESHOLD {
                            engine_view.store.translate_strokes(
                                &self.verticalspace_tool.strokes_below,
                                na::vector![0.0, y_offset],
                            );
                            engine_view.store.translate_strokes_images(
                                &self.verticalspace_tool.strokes_below,
                                na::vector![0.0, y_offset],
                            );

                            self.verticalspace_tool.current_pos_y = element.pos[1];
                        }

                        PenProgress::InProgress
                    }
                    ToolsStyle::OffsetCamera => {
                        let offset = engine_view
                            .camera
                            .transform()
                            .transform_point(&na::Point2::from(element.pos))
                            .coords
                            - engine_view
                                .camera
                                .transform()
                                .transform_point(&na::Point2::from(self.offsetcamera_tool.start))
                                .coords;

                        if offset.magnitude() > 1.0 {
                            engine_view.camera.offset -= offset;

                            engine_view
                                .doc
                                .resize_autoexpand(engine_view.store, engine_view.camera);

                            widget_flags.resize = true;
                            widget_flags.update_view = true;
                        }

                        PenProgress::InProgress
                    }
                };

                widget_flags.redraw = true;
                widget_flags.indicate_changed_store = true;

                pen_progress
            }
            (ToolsState::Active, PenEvent::Up { .. }) => {
                match self.style {
                    ToolsStyle::VerticalSpace => {
                        engine_view
                            .store
                            .update_geometry_for_strokes(&self.verticalspace_tool.strokes_below);
                    }
                    ToolsStyle::OffsetCamera => {}
                }
                engine_view.store.regenerate_rendering_in_viewport_threaded(
                    engine_view.tasks_tx.clone(),
                    false,
                    engine_view.camera.viewport(),
                    engine_view.camera.image_scale(),
                );

                self.reset();
                self.state = ToolsState::Idle;

                engine_view
                    .doc
                    .resize_autoexpand(engine_view.store, engine_view.camera);

                widget_flags.redraw = true;
                widget_flags.resize = true;
                widget_flags.indicate_changed_store = true;
                widget_flags.hide_scrollbars = Some(false);

                PenProgress::Finished
            }
            (ToolsState::Active, PenEvent::Proximity { .. }) => PenProgress::InProgress,
            (ToolsState::Active, PenEvent::KeyPressed { .. }) => PenProgress::InProgress,
            (ToolsState::Active, PenEvent::Cancel) => {
                self.reset();
                self.state = ToolsState::Idle;

                engine_view
                    .doc
                    .resize_autoexpand(engine_view.store, engine_view.camera);

                widget_flags.redraw = true;
                widget_flags.resize = true;
                widget_flags.indicate_changed_store = true;
                widget_flags.hide_scrollbars = Some(false);

                PenProgress::Finished
            }
            (ToolsState::Active, PenEvent::Text { .. }) => PenProgress::InProgress,
        };

        (pen_progress, widget_flags)
    }
}

impl DrawOnDocBehaviour for Tools {
    fn bounds_on_doc(&self, engine_view: &EngineView) -> Option<AABB> {
        match self.state {
            ToolsState::Active => match self.style {
                ToolsStyle::VerticalSpace => self.verticalspace_tool.bounds_on_doc(engine_view),
                ToolsStyle::OffsetCamera => self.offsetcamera_tool.bounds_on_doc(engine_view),
            },
            ToolsState::Idle => None,
        }
    }

    fn draw_on_doc(
        &self,
        cx: &mut piet_cairo::CairoRenderContext,
        engine_view: &EngineView,
    ) -> anyhow::Result<()> {
        cx.save().map_err(|e| anyhow::anyhow!("{}", e))?;

        match &self.style {
            ToolsStyle::VerticalSpace => {
                self.verticalspace_tool.draw_on_doc(cx, engine_view)?;
            }
            ToolsStyle::OffsetCamera => {
                self.offsetcamera_tool.draw_on_doc(cx, engine_view)?;
            }
        }

        cx.restore().map_err(|e| anyhow::anyhow!("{}", e))?;
        Ok(())
    }
}

impl Tools {
    fn reset(&mut self) {
        let current_style = self.style;

        match current_style {
            ToolsStyle::VerticalSpace => {
                self.verticalspace_tool.start_pos_y = 0.0;
                self.verticalspace_tool.current_pos_y = 0.0;
            }
            ToolsStyle::OffsetCamera => {
                self.offsetcamera_tool.start = na::Vector2::zeros();
            }
        }
    }
}
