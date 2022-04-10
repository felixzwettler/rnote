use super::{Stroke, StrokeKey, StrokesState};
use crate::pens::selector::SelectorType;
use crate::pens::Selector;
use crate::render;
use crate::strokes::StrokeBehaviour;
use rnote_compose::helpers::AABBHelpers;

use geo::line_string;
use geo::prelude::*;
use gtk4::{gio, glib, prelude::*};
use p2d::bounding_volume::{BoundingVolume, AABB};
use rayon::prelude::*;
use rnote_compose::shapes::ShapeBehaviour;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(default, rename = "selection_component")]
pub struct SelectionComponent {
    #[serde(default, rename = "selected")]
    pub selected: bool,
}

impl Default for SelectionComponent {
    fn default() -> Self {
        Self { selected: false }
    }
}

impl SelectionComponent {
    pub const SELECTION_DUPLICATION_OFFSET_X: f64 = 20.0;
    pub const SELECTION_DUPLICATION_OFFSET_Y: f64 = 20.0;

    pub fn new(selected: bool) -> Self {
        Self { selected }
    }
}

impl StrokesState {
    /// Returns false if selecting is unsupported
    pub fn can_select(&self, key: StrokeKey) -> bool {
        self.selection_components.get(key).is_some()
    }

    pub fn selected(&self, key: StrokeKey) -> Option<bool> {
        if let Some(selection_comp) = self.selection_components.get(key) {
            Some(selection_comp.selected)
        } else {
            log::debug!(
                "get selection_comp in selected() returned None for stroke with key {:?}",
                key
            );
            None
        }
    }

    /// Sets if the stroke is currently selected
    pub fn set_selected(&mut self, key: StrokeKey, selected: bool) {
        if let Some(selection_comp) = self.selection_components.get_mut(key) {
            selection_comp.selected = selected;

            if let Some(chrono_comp) = self.chrono_components.get_mut(key) {
                self.chrono_counter += 1;
                chrono_comp.t = self.chrono_counter;
            }
        } else {
            log::debug!(
                "get selection_comp in set_selected() returned None for stroke with key {:?}",
                key
            );
        }
    }

    pub fn set_selected_keys(&mut self, keys: &[StrokeKey], selected: bool) {
        keys.iter().for_each(|&key| {
            self.set_selected(key, selected);
        })
    }

    /// Returns the selection keys in the order that they should be rendered. Does not return the stroke keys!
    pub fn selection_keys_as_rendered(&self) -> Vec<StrokeKey> {
        let keys_sorted_chrono = self.keys_sorted_chrono();

        keys_sorted_chrono
            .iter()
            .filter_map(|&key| {
                if self.does_render(key).unwrap_or(false)
                    && !(self.trashed(key).unwrap_or(false))
                    && (self.selected(key).unwrap_or(false))
                {
                    Some(key)
                } else {
                    None
                }
            })
            .collect::<Vec<StrokeKey>>()
    }

    pub fn selection_len(&self) -> usize {
        self.selection_keys_as_rendered().len()
    }

    pub fn gen_selection_bounds(&self) -> Option<AABB> {
        self.gen_bounds(&self.selection_keys_as_rendered())
    }

    pub fn duplicate_selection(&mut self) {
        let offset = na::vector![
            SelectionComponent::SELECTION_DUPLICATION_OFFSET_X,
            SelectionComponent::SELECTION_DUPLICATION_OFFSET_Y
        ];

        let old_selected = self.selection_keys_as_rendered();
        self.set_selected_keys(&old_selected, false);

        let new_selected = old_selected
            .iter()
            .map(|&key| {
                let new_key = self.insert_stroke(self.strokes.get(key).unwrap().clone());
                self.set_selected(new_key, true);
                new_key
            })
            .collect::<Vec<StrokeKey>>();

        // Offsetting the new selected stroke to make the duplication apparent to the user
        self.translate_strokes(&new_selected, offset);
    }

    /// Returns true if selection has changed
    pub fn update_selection_for_selector(&mut self, selector: &Selector, viewport: Option<AABB>) {
        let selector_polygon = match selector.style {
            SelectorType::Polygon => {
                let selector_path_points = selector
                    .path
                    .par_iter()
                    .map(|element| geo::Coordinate {
                        x: element.pos[0],
                        y: element.pos[1],
                    })
                    .collect::<Vec<geo::Coordinate<f64>>>();

                geo::Polygon::new(selector_path_points.into(), vec![])
            }
            SelectorType::Rectangle => {
                if let (Some(first), Some(last)) = (selector.path.first(), selector.path.last()) {
                    let selector_path_points = line_string![
                        (x: first.pos[0], y: first.pos[1]),
                        (x: first.pos[0], y: last.pos[1]),
                        (x: last.pos[0], y: last.pos[1]),
                        (x: last.pos[0], y: first.pos[1]),
                        (x: first.pos[0], y: first.pos[1]),
                    ];

                    geo::Polygon::new(selector_path_points, vec![])
                } else {
                    return;
                }
            }
        };

        self.keys_sorted_chrono().iter().for_each(|&key| {
            let stroke = if let Some(stroke) = self.strokes.get(key) {
                stroke
            } else {
                return;
            };
            // skip if stroke is trashed
            if let Some(trash_comp) = self.trash_components.get(key) {
                if trash_comp.trashed {
                    return;
                }
            }
            // skip if stroke is not in viewport
            if let Some(viewport) = viewport {
                if !viewport.intersects(&stroke.bounds()) {
                    return;
                }
            }
            if let Some(selection_comp) = self.selection_components.get_mut(key) {
                // default to not selected, check for if selected
                selection_comp.selected = false;

                match stroke {
                    Stroke::BrushStroke(brushstroke) => {
                        let brushstroke_bounds = brushstroke.bounds();

                        if selector_polygon.contains(&brushstroke_bounds.to_geo_polygon()) {
                            selection_comp.selected = true;
                        } else if selector_polygon.intersects(&brushstroke_bounds.to_geo_polygon())
                        {
                            for &hitbox_elem in brushstroke.hitboxes.iter() {
                                if !selector_polygon.contains(&hitbox_elem.to_geo_polygon()) {
                                    return;
                                }
                            }
                            selection_comp.selected = true;

                            if let Some(chrono_comp) = self.chrono_components.get_mut(key) {
                                self.chrono_counter += 1;
                                chrono_comp.t = self.chrono_counter;
                            }
                        }
                    }
                    Stroke::ShapeStroke(shapestroke) => {
                        if selector_polygon.contains(&shapestroke.bounds().to_geo_polygon()) {
                            selection_comp.selected = true;

                            if let Some(chrono_comp) = self.chrono_components.get_mut(key) {
                                self.chrono_counter += 1;
                                chrono_comp.t = self.chrono_counter;
                            }
                        }
                    }
                    Stroke::VectorImage(vectorimage) => {
                        if selector_polygon.contains(&vectorimage.bounds().to_geo_polygon()) {
                            selection_comp.selected = true;

                            if let Some(chrono_comp) = self.chrono_components.get_mut(key) {
                                self.chrono_counter += 1;
                                chrono_comp.t = self.chrono_counter;
                            }
                        }
                    }
                    Stroke::BitmapImage(bitmapimage) => {
                        if selector_polygon.contains(&bitmapimage.bounds().to_geo_polygon()) {
                            selection_comp.selected = true;

                            if let Some(chrono_comp) = self.chrono_components.get_mut(key) {
                                self.chrono_counter += 1;
                                chrono_comp.t = self.chrono_counter;
                            }
                        }
                    }
                }
            }
        });
    }

    /// the svgs of the current selection, without xml header or svg root
    pub fn gen_svgs_selection(&self) -> Result<Vec<render::Svg>, anyhow::Error> {
        Ok(self
            .selection_keys_as_rendered()
            .iter()
            .filter_map(|&key| {
                let stroke = self.strokes.get(key)?;

                stroke.gen_svg().ok()
            })
            .collect::<Vec<render::Svg>>())
    }

    pub fn export_selection_as_svg(&self, file: gio::File) -> Result<(), anyhow::Error> {
        let selection_svgs = self.gen_svgs_selection()?;
        let selection_bounds = if let Some(selection_bounds) = self.gen_selection_bounds() {
            selection_bounds
        } else {
            return Ok(());
        };

        let mut svg_data = selection_svgs
            .iter()
            .map(|svg| svg.svg_data.as_str())
            .collect::<Vec<&str>>()
            .join("\n");

        svg_data = rnote_compose::utils::wrap_svg_root(
            svg_data.as_str(),
            Some(selection_bounds),
            Some(selection_bounds),
            true,
        );

        file.replace_async(
            None,
            false,
            gio::FileCreateFlags::REPLACE_DESTINATION,
            glib::PRIORITY_HIGH_IDLE,
            None::<&gio::Cancellable>,
            move |result| {
                let output_stream = match result {
                    Ok(output_stream) => output_stream,
                    Err(e) => {
                        log::error!(
                            "replace_async() failed in export_selection_as_svg() with Err {}",
                            e
                        );
                        return;
                    }
                };

                if let Err(e) = output_stream.write(svg_data.as_bytes(), None::<&gio::Cancellable>)
                {
                    log::error!(
                        "output_stream().write() failed in export_selection_as_svg() with Err {}",
                        e
                    );
                };
                if let Err(e) = output_stream.close(None::<&gio::Cancellable>) {
                    log::error!(
                        "output_stream().close() failed in export_selection_as_svg() with Err {}",
                        e
                    );
                };
            },
        );

        Ok(())
    }
}
