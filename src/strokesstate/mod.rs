pub mod chrono_comp;
pub mod render_comp;
pub mod selection_comp;
pub mod trash_comp;

use std::sync::{Arc, RwLock};

use chrono_comp::ChronoComponent;
use p2d::query::PointQuery;
use render_comp::RenderComponent;
use selection_comp::SelectionComponent;
use trash_comp::TrashComponent;

use crate::compose::geometry;
use crate::compose::transformable::Transformable;
use crate::drawbehaviour::DrawBehaviour;
use crate::pens::tools::DragProximityTool;
use crate::pens::Pens;
use crate::render::{self, Renderer};
use crate::strokes::bitmapimage::BitmapImage;
use crate::strokes::strokestyle::{Element, StrokeStyle};
use crate::strokes::vectorimage::VectorImage;
use crate::ui::appwindow::RnoteAppWindow;

use gtk4::{glib, glib::clone, prelude::*};
use p2d::bounding_volume::{BoundingSphere, BoundingVolume, AABB};
use rayon::iter::{ParallelBridge, ParallelIterator};
use serde::{Deserialize, Serialize};
use slotmap::{HopSlotMap, SecondaryMap};

/*
StrokesState implements a Entity - Component - System pattern.
The Entities are the StrokeKey's, which represent a stroke. There are different components for them:
    * 'strokes': Hold geometric data. These components are special in that they are the primary map. A new stroke must have this component. (could also be called geometric components)
    * 'trash_components': Hold state wether the strokes are trashed
    * 'selection_components': Hold state wether the strokes are selected
    * 'chrono_components': Hold state about the time, chronological ordering
    * 'render_components': Hold state about the current rendering of the strokes.

The systems are implemented as methods on StrokesState, loosely categorized to the different components (but often modify others as well).
Most systems take a key or a slice of keys, and iterate with them over the different components.
There also is a different category of methods which return filtered keys, e.g. `.keys_sorted_chrono` returns the keys in chronological ordering,
    `.stoke_keys_in_order_rendering` returns keys in the order which they should be rendered.
*/

#[derive(Debug, Clone)]
pub enum StateTask {
    UpdateStrokeWithImages {
        key: StrokeKey,
        images: Vec<render::Image>,
    },
    AppendImagesToStroke {
        key: StrokeKey,
        images: Vec<render::Image>,
    },
    InsertStroke {
        stroke: StrokeStyle,
    },
    Quit,
}

pub fn default_threadpool() -> rayon::ThreadPool {
    rayon::ThreadPoolBuilder::default()
        .build()
        .unwrap_or_else(|e| {
            log::error!("default_render_threadpool() failed with Err {}", e);
            panic!()
        })
}

slotmap::new_key_type! {
    pub struct StrokeKey;
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(default, rename = "strokes_state")]
pub struct StrokesState {
    // Components
    #[serde(rename = "strokes")]
    strokes: HopSlotMap<StrokeKey, StrokeStyle>,
    #[serde(rename = "trash_components")]
    trash_components: SecondaryMap<StrokeKey, TrashComponent>,
    #[serde(rename = "selection_components")]
    selection_components: SecondaryMap<StrokeKey, SelectionComponent>,
    #[serde(rename = "chrono_components")]
    chrono_components: SecondaryMap<StrokeKey, ChronoComponent>,
    #[serde(rename = "render_components")]
    render_components: SecondaryMap<StrokeKey, RenderComponent>,

    // Other state
    /// value is equal chrono_component of the newest inserted or modified stroke.
    #[serde(rename = "chrono_counter")]
    chrono_counter: u32,

    #[serde(skip)]
    pub tasks_tx: Option<glib::Sender<StateTask>>,
    #[serde(skip)]
    pub tasks_rx: Option<glib::Receiver<StateTask>>,
    #[serde(skip)]
    pub channel_source: Option<glib::Source>,
    #[serde(skip, default = "default_threadpool")]
    pub threadpool: rayon::ThreadPool,
}

impl Default for StrokesState {
    fn default() -> Self {
        let threadpool = default_threadpool();

        let (render_tx, render_rx) = glib::MainContext::channel::<StateTask>(glib::PRIORITY_HIGH);

        Self {
            strokes: HopSlotMap::with_key(),
            trash_components: SecondaryMap::new(),
            selection_components: SecondaryMap::new(),
            chrono_components: SecondaryMap::new(),
            render_components: SecondaryMap::new(),

            chrono_counter: 0,

            tasks_tx: Some(render_tx),
            tasks_rx: Some(render_rx),
            channel_source: None,
            threadpool,
        }
    }
}

impl StrokesState {
    pub fn new() -> Self {
        Self::default()
    }

    // A new strokes state should always be imported with this method, to not replace the threadpool, channel handlers..
    pub fn import_strokes_state(&mut self, strokes_state: Self) {
        self.strokes = strokes_state.strokes;
        self.trash_components = strokes_state.trash_components;
        self.selection_components = strokes_state.selection_components;
        self.chrono_components = strokes_state.chrono_components;
        self.render_components = strokes_state.render_components;
        self.chrono_counter = strokes_state.chrono_counter;
    }

    /// No self as parameter to avoid already borrowed errors!
    pub fn init(appwindow: &RnoteAppWindow) {
        let main_cx = glib::MainContext::default();

        let source_id = appwindow.canvas().sheet().borrow_mut().strokes_state.tasks_rx.take().unwrap().attach(
            Some(&main_cx),
            clone!(@weak appwindow => @default-return glib::Continue(false), move |render_task| {
                match render_task {
                    StateTask::UpdateStrokeWithImages { key, images } => {
                        appwindow
                            .canvas()
                            .sheet()
                            .borrow_mut()
                            .strokes_state
                            .regenerate_rendering_with_images(key, images, appwindow.canvas().zoom());

                        appwindow.canvas().queue_draw();
                    }
                    StateTask::AppendImagesToStroke { key, images } => {
                        appwindow
                            .canvas()
                            .sheet()
                            .borrow_mut()
                            .strokes_state
                            .append_images_to_rendering(key, images, appwindow.canvas().zoom());

                        appwindow.canvas().queue_draw();
                    }
                    StateTask::InsertStroke { stroke } => {
                        match stroke {
                            StrokeStyle::MarkerStroke(markerstroke) => {
                                let inserted = appwindow.canvas().sheet()
                                    .borrow_mut()
                                    .strokes_state
                                    .insert_stroke(StrokeStyle::MarkerStroke(markerstroke));

                                appwindow.canvas().sheet()
                                    .borrow_mut()
                                    .strokes_state
                                    .regenerate_rendering_for_stroke_threaded(inserted, appwindow.canvas().renderer(), appwindow.canvas().zoom());
                            }
                            StrokeStyle::BrushStroke(brushstroke) => {
                                let inserted = appwindow.canvas().sheet()
                                    .borrow_mut()
                                    .strokes_state
                                    .insert_stroke(StrokeStyle::BrushStroke(brushstroke));

                                appwindow.canvas().sheet()
                                    .borrow_mut()
                                    .strokes_state
                                    .regenerate_rendering_for_stroke_threaded(inserted, appwindow.canvas().renderer(), appwindow.canvas().zoom());
                            }
                            StrokeStyle::ShapeStroke(shapestroke) => {
                                let inserted = appwindow.canvas().sheet()
                                    .borrow_mut()
                                    .strokes_state
                                    .insert_stroke(StrokeStyle::ShapeStroke(shapestroke));

                                appwindow.canvas().sheet()
                                    .borrow_mut()
                                    .strokes_state
                                    .regenerate_rendering_for_stroke_threaded(inserted, appwindow.canvas().renderer(), appwindow.canvas().zoom());
                            }
                            StrokeStyle::VectorImage(vectorimage) => {
                                let inserted = appwindow.canvas().sheet()
                                    .borrow_mut()
                                    .strokes_state
                                    .insert_stroke(StrokeStyle::VectorImage(vectorimage));
                                appwindow.canvas().sheet()
                                    .borrow_mut()
                                    .strokes_state
                                    .set_selected(inserted, true);

                                appwindow.canvas().sheet()
                                    .borrow_mut()
                                    .strokes_state
                                    .regenerate_rendering_for_stroke_threaded(inserted, appwindow.canvas().renderer(), appwindow.canvas().zoom());

                                appwindow.mainheader().selector_toggle().set_active(true);

                                appwindow.canvas().resize_sheet_to_fit_strokes();
                                appwindow.canvas().selection_modifier().update_state(&appwindow.canvas());
                            }
                            StrokeStyle::BitmapImage(bitmapimage) => {
                                let inserted = appwindow
                                    .canvas()
                                    .sheet()
                                    .borrow_mut()
                                    .strokes_state
                                    .insert_stroke(StrokeStyle::BitmapImage(bitmapimage));

                                appwindow.canvas().sheet()
                                    .borrow_mut()
                                    .strokes_state
                                    .set_selected(inserted, true);

                                appwindow.canvas().sheet()
                                    .borrow_mut()
                                    .strokes_state
                                    .regenerate_rendering_for_stroke_threaded(inserted, appwindow.canvas().renderer(), appwindow.canvas().zoom());

                                appwindow.mainheader().selector_toggle().set_active(true);

                                appwindow.canvas().resize_sheet_to_fit_strokes();
                                appwindow.canvas().selection_modifier().update_state(&appwindow.canvas());
                            }
                        }

                        appwindow.canvas().queue_resize();
                        appwindow.canvas().queue_draw();
                    }
                    StateTask::Quit => {
                        return glib::Continue(false);
                    }
                }

                glib::Continue(true)
            }),
        );

        let source = main_cx.find_source_by_id(&source_id).unwrap_or_else(|| {
            log::error!("find_source_by_id() in StrokeState init() failed.");
            panic!();
        });
        appwindow
            .canvas()
            .sheet()
            .borrow_mut()
            .strokes_state
            .channel_source
            .replace(source);
    }

    pub fn insert_stroke(&mut self, stroke: StrokeStyle) -> StrokeKey {
        let key = self.strokes.insert(stroke);
        self.chrono_counter += 1;

        self.trash_components.insert(key, TrashComponent::default());
        self.selection_components
            .insert(key, SelectionComponent::default());
        self.render_components
            .insert(key, RenderComponent::default());
        self.chrono_components
            .insert(key, ChronoComponent::new(self.chrono_counter));

        // set flag for rendering regeneration
        if let Some(render_comp) = self.render_components.get_mut(key) {
            render_comp.regenerate_flag = true;
        }
        key
    }

    pub fn remove_stroke(&mut self, key: StrokeKey) -> Option<StrokeStyle> {
        self.trash_components.remove(key);
        self.selection_components.remove(key);
        self.chrono_components.remove(key);
        self.render_components.remove(key);

        self.strokes.remove(key)
    }

    pub fn add_to_stroke(
        &mut self,
        key: StrokeKey,
        pens: &mut Pens,
        element: Element,
        renderer: Arc<RwLock<Renderer>>,
        zoom: f64,
    ) {
        match self.strokes.get_mut(key).unwrap() {
            StrokeStyle::MarkerStroke(ref mut markerstroke) => {
                markerstroke.push_elem(element);
            }
            StrokeStyle::BrushStroke(ref mut brushstroke) => {
                brushstroke.push_elem(element);
            }
            StrokeStyle::ShapeStroke(ref mut shapestroke) => {
                shapestroke.update_shape(&mut pens.shaper, element);
            }
            StrokeStyle::VectorImage(_vectorimage) => {}
            StrokeStyle::BitmapImage(_bitmapimage) => {}
        }

        self.append_rendering_new_elem_threaded(key, renderer, zoom);
    }

    /// Clears every stroke and every component
    pub fn clear(&mut self) {
        self.chrono_counter = 0;

        self.strokes.clear();
        self.trash_components.clear();
        self.selection_components.clear();
        self.chrono_components.clear();
        self.render_components.clear();
    }

    /// Returns the stroke keys in the order that they should be rendered. Does not return the selection keys!
    pub fn stroke_keys_in_order_rendered(&self) -> Vec<StrokeKey> {
        let keys_sorted_chrono = self.keys_sorted_chrono();

        keys_sorted_chrono
            .iter()
            .filter_map(|&key| {
                if self.does_render(key).unwrap_or(false)
                    && !(self.trashed(key).unwrap_or(false))
                    && !(self.selected(key).unwrap_or(false))
                {
                    Some(key)
                } else {
                    None
                }
            })
            .collect::<Vec<StrokeKey>>()
    }

    pub fn stroke_keys_intersect_bounds(&self, bounds: AABB) -> Vec<StrokeKey> {
        self.stroke_keys_in_order_rendered()
            .iter()
            .filter_map(|&key| {
                let stroke = self.strokes.get(key)?;
                if stroke.bounds().intersects(&bounds) {
                    Some(key)
                } else {
                    None
                }
            })
            .collect::<Vec<StrokeKey>>()
    }

    pub fn clone_strokes_for_keys(&self, keys: &[StrokeKey]) -> Vec<StrokeStyle> {
        keys.iter()
            .filter_map(|&key| Some(self.strokes.get(key)?.clone()))
            .collect::<Vec<StrokeStyle>>()
    }

    pub fn insert_vectorimage_bytes_threaded(
        &mut self,
        pos: na::Vector2<f64>,
        bytes: glib::Bytes,
        renderer: Arc<RwLock<Renderer>>,
    ) {
        if let Some(tasks_tx) = self.tasks_tx.clone() {
            self.threadpool.spawn(move || {
                match String::from_utf8(bytes.to_vec()) {
                    Ok(svg) => {
                        match VectorImage::import_from_svg_data(svg.as_str(), pos, None, renderer) {
                            Ok(vectorimage) => {
                                let vectorimage = StrokeStyle::VectorImage(vectorimage);

                                tasks_tx.send(StateTask::InsertStroke {
                                    stroke: vectorimage
                                }).unwrap_or_else(|e| {
                                    log::error!("tasks_tx.send() failed in insert_vectorimage_bytes_threaded() with Err, {}", e);
                                });
                            }
                            Err(e) => {
                                log::error!("VectorImage::import_from_svg_data() failed in insert_vectorimage_bytes_threaded() with Err, {}", e);
                            }
                        }
                    }
                    Err(e) => {
                        log::error!("from_utf8() failed in thread from insert_vectorimages_bytes_threaded() with Err {}", e);
                    }
                }
            });
        }
    }

    pub fn insert_bitmapimage_bytes_threaded(&mut self, pos: na::Vector2<f64>, bytes: glib::Bytes) {
        if let Some(tasks_tx) = self.tasks_tx.clone() {
            self.threadpool.spawn(move || {
                match BitmapImage::import_from_image_bytes(bytes, pos) {
                    Ok(bitmapimage) => {
                        let bitmapimage = StrokeStyle::BitmapImage(bitmapimage);

                        tasks_tx.send(StateTask::InsertStroke {
                            stroke: bitmapimage
                        }).unwrap_or_else(|e| {
                            log::error!("tasks_tx.send() failed in insert_bitmapimage_bytes_threaded() with Err, {}", e);
                        });
                    }
                    Err(e) => {
                        log::error!("BitmapImage::import_from_svg_data() failed in insert_bitmapimage_bytes_threaded() with Err, {}", e);
                    }
                }
            });
        }
    }

    pub fn insert_pdf_bytes_as_vector_threaded(
        &mut self,
        pos: na::Vector2<f64>,
        page_width: Option<i32>,
        bytes: glib::Bytes,
        renderer: Arc<RwLock<Renderer>>,
    ) {
        if let Some(tasks_tx) = self.tasks_tx.clone() {
            self.threadpool.spawn(move || {
                match VectorImage::import_from_pdf_bytes(&bytes, pos, page_width, renderer) {
                    Ok(images) => {
                        for image in images {
                            let image = StrokeStyle::VectorImage(image);

                            tasks_tx.send(StateTask::InsertStroke {
                                stroke: image
                            }).unwrap_or_else(|e| {
                                log::error!("tasks_tx.send() failed in insert_pdf_bytes_as_vector_threaded() with Err, {}", e);
                            });
                        }
                    }
                    Err(e) => {
                        log::error!("VectorImage::import_from_pdf_bytes() failed in insert_pdf_bytes_as_vector_threaded() with Err, {}", e);
                    }
                }
            });
        }
    }

    pub fn insert_pdf_bytes_as_bitmap_threaded(
        &mut self,
        pos: na::Vector2<f64>,
        page_width: Option<i32>,
        bytes: glib::Bytes,
    ) {
        if let Some(tasks_tx) = self.tasks_tx.clone() {
            self.threadpool.spawn(move || {
                match BitmapImage::import_from_pdf_bytes(&bytes, pos, page_width) {
                    Ok(images) => {
                        for image in images {
                            let image = StrokeStyle::BitmapImage(image);

                            tasks_tx.send(StateTask::InsertStroke {
                                stroke: image
                            }).unwrap_or_else(|e| {
                                log::error!("tasks_tx.send() failed in insert_pdf_bytes_as_bitmap_threaded() with Err, {}", e);
                            });
                        }
                    }
                    Err(e) => {
                        log::error!("BitmapImage::import_from_pdf_bytes() failed in insert_pdf_bytes_as_bitmap_threaded() with Err, {}", e);
                    }
                }
            });
        }
    }

    pub fn import_state(&mut self, strokes_state: &Self) {
        self.clear();
        self.chrono_counter = strokes_state.chrono_counter;

        self.strokes = strokes_state.strokes.clone();
        self.trash_components = strokes_state.trash_components.clone();
        self.selection_components = strokes_state.selection_components.clone();
        self.chrono_components = strokes_state.chrono_components.clone();
        self.render_components = strokes_state.render_components.clone();
    }

    pub fn update_geometry_for_stroke(&mut self, key: StrokeKey) {
        if let Some(stroke) = self.strokes.get_mut(key) {
            match stroke {
                StrokeStyle::MarkerStroke(ref mut markerstroke) => {
                    markerstroke.update_geometry();
                }
                StrokeStyle::BrushStroke(ref mut brushstroke) => {
                    brushstroke.update_geometry();
                }
                StrokeStyle::ShapeStroke(shapestroke) => {
                    shapestroke.update_geometry();
                }
                StrokeStyle::VectorImage(ref mut vectorimage) => {
                    vectorimage.update_geometry();
                }
                StrokeStyle::BitmapImage(ref mut bitmapimage) => {
                    bitmapimage.update_geometry();
                }
            }

            // set flag for rendering regeneration
            if let Some(render_comp) = self.render_components.get_mut(key) {
                render_comp.regenerate_flag = true;
            }
        } else {
            log::debug!(
                "get stroke in update_stroke_geometry() returned None in complete_stroke() for key {:?}",
                key
            );
        }
    }

    pub fn update_geometry_all_strokes(&mut self) {
        let keys: Vec<StrokeKey> = self.strokes.keys().collect();

        keys.iter().for_each(|&key| {
            self.update_geometry_for_stroke(key);
        });
    }

    pub fn update_geometry_selection_strokes(&mut self) {
        let keys: Vec<StrokeKey> = self.selection_keys_in_order_rendered();

        keys.iter().for_each(|&key| {
            self.update_geometry_for_stroke(key);
        });
    }

    /// Calculates the width needed to fit all strokes
    pub fn calc_width(&self) -> f64 {
        let new_width = if let Some(stroke) = self
            .strokes
            .iter()
            .filter_map(|(key, stroke)| {
                if let Some(trash_comp) = self.trash_components.get(key) {
                    if !trash_comp.trashed {
                        return Some(stroke);
                    }
                }
                None
            })
            .max_by_key(|&stroke| stroke.bounds().maxs[0].round() as i32)
        {
            // max_by_key() returns the element, so we need to extract the width again
            stroke.bounds().maxs[0]
        } else {
            0.0
        };

        new_width
    }

    /// Calculates the height needed to fit all strokes
    pub fn calc_height(&self) -> f64 {
        let new_height = if let Some(stroke) = self
            .strokes
            .iter()
            .filter_map(|(key, stroke)| {
                if let Some(trash_comp) = self.trash_components.get(key) {
                    if !trash_comp.trashed {
                        return Some(stroke);
                    }
                }
                None
            })
            .max_by_key(|&stroke| stroke.bounds().maxs[1].round() as i32)
        {
            // max_by_key() returns the element, so we need to extract the height again
            stroke.bounds().maxs[1]
        } else {
            0.0
        };

        new_height
    }

    /// Generates the bounds which enclose the strokes
    pub fn gen_bounds(&self, keys: &[StrokeKey]) -> Option<AABB> {
        let mut keys_iter = keys.iter();
        if let Some(&key) = keys_iter.next() {
            if let Some(first) = self.strokes.get(key) {
                let mut bounds = first.bounds();

                keys_iter
                    .filter_map(|&key| self.strokes.get(key))
                    .for_each(|stroke| {
                        bounds.merge(&stroke.bounds());
                    });

                return Some(bounds);
            }
        }

        None
    }

    pub fn strokes_bounds(&self, keys: &[StrokeKey]) -> Vec<AABB> {
        keys.iter()
            .filter_map(|&key| Some(self.strokes.get(key)?.bounds()))
            .collect::<Vec<AABB>>()
    }

    /// Generates a Svg for all strokes as drawn onto the canvas without xml headers or svg roots. Does not include the selection.
    pub fn gen_svgs_for_strokes(&self) -> Result<Vec<render::Svg>, anyhow::Error> {
        let chrono_sorted = self.keys_sorted_chrono();

        let svgs = chrono_sorted
            .iter()
            .filter(|&&key| {
                self.does_render(key).unwrap_or(false)
                    && !(self.trashed(key).unwrap_or(false))
                    && !(self.selected(key).unwrap_or(false))
                    && (self.does_render(key).unwrap_or(false))
            })
            .filter_map(|&key| {
                let stroke = self.strokes.get(key)?;

                match stroke.gen_svgs(na::vector![0.0, 0.0]) {
                    Ok(svgs) => Some(svgs),
                    Err(e) => {
                        log::error!(
                            "stroke.gen_svgs() failed in gen_svg_all_strokes() with Err {}",
                            e
                        );
                        None
                    }
                }
            })
            .flatten()
            .collect::<Vec<render::Svg>>();

        Ok(svgs)
    }

    /// Translate the strokes with the offset
    pub fn translate_strokes(
        &mut self,
        strokes: &[StrokeKey],
        offset: na::Vector2<f64>,
        zoom: f64,
    ) {
        strokes.iter().for_each(|&key| {
            if let Some(stroke) = self.strokes.get_mut(key) {
                stroke.translate(offset);

                if let Some(render_comp) = self.render_components.get_mut(key) {
                    for image in render_comp.images.iter_mut() {
                        image.bounds = geometry::aabb_translate(image.bounds, offset);
                    }

                    match render::images_to_rendernode(&render_comp.images, zoom) {
                        Ok(Some(rendernode)) => {
                            render_comp.rendernode = Some(rendernode);
                        }
                        Ok(None) => {}
                        Err(e) => log::error!(
                            "images_to_rendernode() failed in translate_strokes() with Err {}",
                            e
                        ),
                    }
                }
            }
        });
    }

    /// Rotates the stroke with angle (rad) around the center
    pub fn rotate_strokes(
        &mut self,
        strokes: &[StrokeKey],
        angle: f64,
        center: na::Point2<f64>,
        renderer: Arc<RwLock<Renderer>>,
        zoom: f64,
    ) {
        strokes.iter().for_each(|&key| {
            if let Some(stroke) = self.strokes.get_mut(key) {
                stroke.rotate(angle, center);

                self.regenerate_rendering_for_stroke(key, Arc::clone(&renderer), zoom);
            }
        });
    }

    // Resizes the strokes to new bounds
    pub fn resize_strokes(
        &mut self,
        strokes: &[StrokeKey],
        old_bounds: AABB,
        new_bounds: AABB,
        renderer: Arc<RwLock<Renderer>>,
        zoom: f64,
    ) {
        strokes.iter().for_each(|&key| {
            if let Some(stroke) = self.strokes.get_mut(key) {
                let old_stroke_bounds = stroke.bounds();
                let new_stroke_bounds = geometry::scale_inner_bounds_to_new_outer_bounds(
                    stroke.bounds(),
                    old_bounds,
                    new_bounds,
                );

                let offset = new_stroke_bounds.center() - old_stroke_bounds.center();
                let scale = new_stroke_bounds
                    .extents()
                    .component_div(&old_stroke_bounds.extents());

                stroke.translate(offset);
                stroke.scale(scale);

                self.regenerate_rendering_for_stroke(key, Arc::clone(&renderer), zoom);
            }
        });
    }

    /// Returns all strokes below the y_pos
    pub fn strokes_below_y_pos(&self, y_pos: f64) -> Vec<StrokeKey> {
        self.strokes
            .iter()
            .filter_map(|(key, stroke)| {
                if stroke.bounds().mins[1] > y_pos {
                    Some(key)
                } else {
                    None
                }
            })
            .collect::<Vec<StrokeKey>>()
    }

    pub fn drag_strokes_proximity(
        &mut self,
        drag_proximity_tool: &DragProximityTool,
        renderer: Arc<RwLock<Renderer>>,
        zoom: f64,
    ) {
        let sphere = BoundingSphere {
            center: na::Point2::from(drag_proximity_tool.pos),
            radius: drag_proximity_tool.radius,
        };
        let tool_bounds = geometry::aabb_new_positive(
            na::point![
                drag_proximity_tool.pos[0] - drag_proximity_tool.radius,
                drag_proximity_tool.pos[1] - drag_proximity_tool.radius
            ],
            na::point![
                drag_proximity_tool.pos[0] + drag_proximity_tool.radius,
                drag_proximity_tool.pos[1] + drag_proximity_tool.radius
            ],
        );

        self.strokes
            .iter_mut()
            .par_bridge()
            .filter_map(|(key, stroke)| match stroke {
                StrokeStyle::MarkerStroke(markerstroke) => {
                    if markerstroke.bounds().intersects(&tool_bounds) {
                        markerstroke.elements.iter_mut().for_each(|element| {
                            if sphere
                                .contains_local_point(&na::Point2::from(element.inputdata.pos()))
                            {
                                // Zero when right at drag_proximity_tool position, One when right at the radius
                                let distance_ratio = (1.0
                                    - (element.inputdata.pos() - drag_proximity_tool.pos)
                                        .magnitude()
                                        / drag_proximity_tool.radius)
                                    .clamp(0.0, 1.0);

                                element.inputdata.set_pos(
                                    element.inputdata.pos()
                                        + drag_proximity_tool.offset * distance_ratio,
                                );
                            }
                        });
                        Some(key)
                    } else {
                        None
                    }
                }
                StrokeStyle::BrushStroke(brushstroke) => {
                    if brushstroke.bounds().intersects(&tool_bounds) {
                        brushstroke.elements.iter_mut().for_each(|element| {
                            if sphere
                                .contains_local_point(&na::Point2::from(element.inputdata.pos()))
                            {
                                // Zero when right at drag_proximity_tool position, One when right at the radius
                                let distance_ratio = (1.0
                                    - (element.inputdata.pos() - drag_proximity_tool.pos)
                                        .magnitude()
                                        / drag_proximity_tool.radius)
                                    .clamp(0.0, 1.0);

                                element.inputdata.set_pos(
                                    element.inputdata.pos()
                                        + drag_proximity_tool.offset * distance_ratio,
                                );
                            }
                        });
                        Some(key)
                    } else {
                        None
                    }
                }
                _ => None,
            })
            .collect::<Vec<StrokeKey>>()
            .iter()
            .for_each(|&key| {
                self.update_geometry_for_stroke(key);
                self.regenerate_rendering_for_stroke(key, Arc::clone(&renderer), zoom);
            });
    }
}
