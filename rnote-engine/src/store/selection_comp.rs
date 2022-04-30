use std::sync::Arc;

use super::{StrokeKey, StrokeStore};

use geo::prelude::*;
use p2d::bounding_volume::{BoundingVolume, AABB};
use rayon::prelude::*;
use rnote_compose::penpath::Element;
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

impl StrokeStore {
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
        if let Some(selection_comp) = Arc::make_mut(&mut self.selection_components)
            .get_mut(key)
            .map(Arc::make_mut)
        {
            selection_comp.selected = selected;

            self.update_chrono_to_last(key);
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

    pub fn selection_keys_unordered(&self) -> Vec<StrokeKey> {
        self.stroke_components
            .keys()
            .filter(|&key| {
                !(self.trashed(key).unwrap_or(false)) && (self.selected(key).unwrap_or(false))
            })
            .collect()
    }

    /// Returns the selection keys in the order that they should be rendered. Does not return the stroke keys!
    pub fn selection_keys_as_rendered(&self) -> Vec<StrokeKey> {
        let keys_sorted_chrono = self.keys_sorted_chrono();

        keys_sorted_chrono
            .into_iter()
            .filter(|&key| {
                !(self.trashed(key).unwrap_or(false)) && (self.selected(key).unwrap_or(false))
            })
            .collect::<Vec<StrokeKey>>()
    }

    pub fn selection_keys_as_rendered_intersecting_bounds(&self, bounds: AABB) -> Vec<StrokeKey> {
        self.keys_sorted_chrono_intersecting_bounds(bounds)
            .into_iter()
            .filter(|&key| {
                !(self.trashed(key).unwrap_or(false)) && (self.selected(key).unwrap_or(false))
            })
            .collect::<Vec<StrokeKey>>()
    }

    pub fn selection_len(&self) -> usize {
        self.selection_keys_unordered().len()
    }

    pub fn gen_selection_bounds(&self) -> Option<AABB> {
        self.gen_bounds_for_strokes(&self.selection_keys_unordered())
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
            .filter_map(|&key| {
                let new_key = self.insert_stroke((**self.stroke_components.get(key)?).clone());
                self.set_selected(new_key, true);
                Some(new_key)
            })
            .collect::<Vec<StrokeKey>>();

        // Offsetting the new selected stroke to make the duplication apparent to the user
        self.translate_strokes(&new_selected, offset);
    }

    /// selects the strokes intersecting a given polygon path. Returns the new selected keys
    pub fn select_keys_intersecting_polygon_path(
        &mut self,
        path: &[Element],
        viewport: AABB,
    ) -> Vec<StrokeKey> {
        let selector_polygon = {
            let selector_path_points = path
                .par_iter()
                .map(|element| geo::Coordinate {
                    x: element.pos[0],
                    y: element.pos[1],
                })
                .collect::<Vec<geo::Coordinate<f64>>>();

            geo::Polygon::new(selector_path_points.into(), vec![])
        };

        self.keys_sorted_chrono_intersecting_bounds(viewport)
            .into_iter()
            .filter_map(|key| {
                // skip if stroke is trashed
                if self.trashed(key)? {
                    return None;
                }

                let stroke = self.stroke_components.get(key)?;
                let stroke_bounds = stroke.bounds();

                if selector_polygon.contains(&crate::utils::p2d_aabb_to_geo_polygon(stroke_bounds))
                {
                    self.set_selected(key, true);
                    self.update_chrono_to_last(key);

                    return Some(key);
                } else if selector_polygon
                    .intersects(&crate::utils::p2d_aabb_to_geo_polygon(stroke_bounds))
                {
                    for &hitbox_elem in stroke.hitboxes().iter() {
                        if !selector_polygon
                            .contains(&crate::utils::p2d_aabb_to_geo_polygon(hitbox_elem))
                        {
                            return None;
                        }
                    }

                    self.set_selected(key, true);
                    self.update_chrono_to_last(key);

                    return Some(key);
                }

                None
            })
            .collect()
    }

    /// selects the strokes intersecting a given aabb. Returns the new selected keys
    pub fn select_keys_intersecting_aabb(&mut self, aabb: AABB, viewport: AABB) -> Vec<StrokeKey> {
        self.keys_sorted_chrono_intersecting_bounds(viewport)
            .into_iter()
            .filter_map(|key| {
                // skip if stroke is trashed
                if self.trashed(key)? {
                    return None;
                }

                let stroke = self.stroke_components.get(key)?;
                let stroke_bounds = stroke.bounds();

                if aabb.contains(&stroke_bounds) {
                    self.set_selected(key, true);
                    self.update_chrono_to_last(key);

                    return Some(key);
                } else if aabb.intersects(&stroke_bounds) {
                    for &hitbox_elem in stroke.hitboxes().iter() {
                        if !aabb.contains(&hitbox_elem) {
                            return None;
                        }
                    }

                    self.set_selected(key, true);
                    self.update_chrono_to_last(key);

                    return Some(key);
                }

                None
            })
            .collect()
    }
}
