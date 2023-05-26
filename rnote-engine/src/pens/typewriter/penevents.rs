// Imports
use super::{ModifyState, Typewriter, TypewriterState};
use crate::engine::EngineViewMut;
use crate::pens::penbehaviour::PenProgress;
use crate::pens::PenBehaviour;
use crate::strokes::{Stroke, TextStroke};
use crate::{DrawOnDocBehaviour, StrokeStore, WidgetFlags};
use rnote_compose::penevents::{KeyboardKey, ModifierKey};
use rnote_compose::penpath::Element;
use std::time::Instant;
use unicode_segmentation::GraphemeCursor;

impl Typewriter {
    pub(super) fn handle_pen_event_down(
        &mut self,
        element: Element,
        _modifier_keys: Vec<ModifierKey>,
        _now: Instant,
        engine_view: &mut EngineViewMut,
    ) -> (PenProgress, WidgetFlags) {
        let mut widget_flags = WidgetFlags::default();
        let typewriter_bounds = self.bounds_on_doc(&engine_view.as_im());
        let text_width = engine_view.pens_config.typewriter_config.text_width;

        let pen_progress = match &mut self.state {
            TypewriterState::Idle | TypewriterState::Start { .. } => {
                let mut refresh_state = false;
                let mut new_state = TypewriterState::Start(element.pos);

                if let Some(&stroke_key) = engine_view
                    .store
                    .stroke_hitboxes_contain_coord(engine_view.camera.viewport(), element.pos)
                    .last()
                {
                    // When clicked on a textstroke, we start modifying it
                    if let Some(Stroke::TextStroke(textstroke)) =
                        engine_view.store.get_stroke_ref(stroke_key)
                    {
                        let cursor = if let Ok(new_cursor) =
                            // get the cursor for the current position
                            textstroke.get_cursor_for_global_coord(element.pos)
                        {
                            new_cursor
                        } else {
                            GraphemeCursor::new(0, textstroke.text.len(), true)
                        };

                        engine_view.store.update_chrono_to_last(stroke_key);

                        new_state = TypewriterState::Modifying {
                            modify_state: ModifyState::Up,
                            stroke_key,
                            cursor,
                            pen_down: true,
                        };
                        refresh_state = true;
                    }
                }

                self.state = new_state;

                // after setting new state
                if refresh_state {
                    // Update typewriter state for the current textstroke, and indicate that the penholder has changed, to update the UI
                    widget_flags.merge(self.update_state(engine_view));
                    widget_flags.refresh_ui = true;
                }

                PenProgress::InProgress
            }
            TypewriterState::Modifying {
                modify_state,
                stroke_key,
                cursor,
                pen_down,
            } => {
                match modify_state {
                    ModifyState::Up | ModifyState::Hover(_) => {
                        let mut progress = PenProgress::InProgress;

                        if let (Some(typewriter_bounds), Some(Stroke::TextStroke(textstroke))) = (
                            typewriter_bounds,
                            engine_view.store.get_stroke_ref(*stroke_key),
                        ) {
                            if Self::translate_node_bounds(typewriter_bounds, engine_view.camera)
                                .contains_local_point(&na::Point2::from(element.pos))
                            {
                                // switch to translating state
                                self.state = TypewriterState::Modifying {
                                    modify_state: ModifyState::Translating {
                                        current_pos: element.pos,
                                    },
                                    stroke_key: *stroke_key,
                                    cursor: cursor.clone(),
                                    pen_down: true,
                                };
                            } else if Self::adjust_text_width_node_bounds(
                                Self::text_rect_bounds(text_width, textstroke).mins.coords,
                                text_width,
                                engine_view.camera,
                            )
                            .contains_local_point(&na::Point2::from(element.pos))
                            {
                                // switch to adjust text width
                                self.state = TypewriterState::Modifying {
                                    modify_state: ModifyState::AdjustTextWidth {
                                        start_text_width: text_width,
                                        start_pos: element.pos,
                                        current_pos: element.pos,
                                    },
                                    stroke_key: *stroke_key,
                                    cursor: cursor.clone(),
                                    pen_down: true,
                                };
                            // This is intentionally **not** the textstroke hitboxes
                            } else if typewriter_bounds
                                .contains_local_point(&na::Point2::from(element.pos))
                            {
                                if let Some(Stroke::TextStroke(textstroke)) =
                                    engine_view.store.get_stroke_ref(*stroke_key)
                                {
                                    if let Ok(new_cursor) =
                                        textstroke.get_cursor_for_global_coord(element.pos)
                                    {
                                        if new_cursor.cur_cursor() != cursor.cur_cursor()
                                            && *pen_down
                                        {
                                            // switch to selecting state
                                            self.state = TypewriterState::Modifying {
                                                modify_state: ModifyState::Selecting {
                                                    selection_cursor: cursor.clone(),
                                                    finished: false,
                                                },
                                                stroke_key: *stroke_key,
                                                cursor: cursor.clone(),
                                                pen_down: true,
                                            };
                                        } else {
                                            *cursor = new_cursor;
                                            *pen_down = true;
                                        }
                                    }
                                }
                            } else {
                                // If we click outside, reset to idle
                                self.state = TypewriterState::Idle;
                                progress = PenProgress::Finished;
                            }
                        }

                        progress
                    }
                    ModifyState::Selecting { finished, .. } => {
                        let mut progress = PenProgress::InProgress;

                        if let Some(typewriter_bounds) = typewriter_bounds {
                            // Clicking on the translate node
                            if Self::translate_node_bounds(typewriter_bounds, engine_view.camera)
                                .contains_local_point(&na::Point2::from(element.pos))
                            {
                                self.state = TypewriterState::Modifying {
                                    modify_state: ModifyState::Translating {
                                        current_pos: element.pos,
                                    },
                                    stroke_key: *stroke_key,
                                    cursor: cursor.clone(),
                                    pen_down: true,
                                };
                            } else if typewriter_bounds
                                .contains_local_point(&na::Point2::from(element.pos))
                            {
                                if let Some(Stroke::TextStroke(textstroke)) =
                                    engine_view.store.get_stroke_ref(*stroke_key)
                                {
                                    if *finished {
                                        if let Ok(new_cursor) =
                                            textstroke.get_cursor_for_global_coord(element.pos)
                                        {
                                            // If selecting is finished, return to modifying with the current pen position as cursor
                                            self.state = TypewriterState::Modifying {
                                                modify_state: ModifyState::Up,
                                                stroke_key: *stroke_key,
                                                cursor: new_cursor,
                                                pen_down: true,
                                            };
                                        }
                                    } else {
                                        // Updating the cursor for the clicked position
                                        if let Ok(new_cursor) =
                                            textstroke.get_cursor_for_global_coord(element.pos)
                                        {
                                            *cursor = new_cursor
                                        }
                                    }
                                }
                            } else {
                                // If we click outside, reset to idle
                                self.state = TypewriterState::Idle;
                                progress = PenProgress::Finished;
                            }
                        }

                        progress
                    }
                    ModifyState::Translating { current_pos, .. } => {
                        let offset = element.pos - *current_pos;

                        if offset.magnitude()
                            > Self::TRANSLATE_MAGNITUDE_THRESHOLD / engine_view.camera.total_zoom()
                        {
                            engine_view.store.translate_strokes(&[*stroke_key], offset);
                            engine_view
                                .store
                                .translate_strokes_images(&[*stroke_key], offset);

                            *current_pos = element.pos;

                            widget_flags.store_modified = true;
                        }

                        PenProgress::InProgress
                    }
                    ModifyState::AdjustTextWidth {
                        start_text_width,
                        start_pos,
                        current_pos,
                    } => {
                        let x_offset = element.pos[0] - current_pos[0];

                        if let Some(Stroke::TextStroke(textstroke)) =
                            engine_view.store.get_stroke_mut(*stroke_key)
                        {
                            if x_offset.abs()
                                > Self::ADJ_TEXT_WIDTH_THRESHOLD / engine_view.camera.total_zoom()
                            {
                                let abs_x_offset = element.pos[0] - start_pos[0];
                                engine_view.pens_config.typewriter_config.text_width =
                                    (*start_text_width + abs_x_offset).max(2.0);
                                if let Some(max_width) = &mut textstroke.text_style.max_width {
                                    *max_width = *start_text_width + abs_x_offset;
                                }
                                engine_view.store.regenerate_rendering_for_stroke(
                                    *stroke_key,
                                    engine_view.camera.viewport(),
                                    engine_view.camera.image_scale(),
                                );

                                *current_pos = element.pos;

                                widget_flags.store_modified = true;
                            }
                        }

                        PenProgress::InProgress
                    }
                }
            }
        };

        (pen_progress, widget_flags)
    }

    pub(super) fn handle_pen_event_up(
        &mut self,
        element: Element,
        _modifier_keys: Vec<ModifierKey>,
        _now: Instant,
        engine_view: &mut EngineViewMut,
    ) -> (PenProgress, WidgetFlags) {
        let mut widget_flags = WidgetFlags::default();
        let typewriter_bounds = self.bounds_on_doc(&engine_view.as_im());

        let pen_progress = match &mut self.state {
            TypewriterState::Idle => PenProgress::Idle,
            TypewriterState::Start(_) => PenProgress::InProgress,
            TypewriterState::Modifying {
                modify_state,
                stroke_key,
                cursor,
                pen_down,
                ..
            } => {
                *pen_down = false;

                match modify_state {
                    ModifyState::Up | ModifyState::Hover(_) => {
                        // detect hover state
                        *modify_state = if typewriter_bounds
                            .map(|b| b.contains_local_point(&na::Point2::from(element.pos)))
                            .unwrap_or(false)
                        {
                            ModifyState::Hover(element.pos)
                        } else {
                            ModifyState::Up
                        }
                    }
                    ModifyState::Selecting { finished, .. } => {
                        // finished when drag ended
                        *finished = true;
                    }
                    ModifyState::Translating { .. } => {
                        engine_view
                            .store
                            .update_geometry_for_strokes(&[*stroke_key]);
                        engine_view.store.regenerate_rendering_for_stroke(
                            *stroke_key,
                            engine_view.camera.viewport(),
                            engine_view.camera.image_scale(),
                        );
                        engine_view
                            .doc
                            .resize_autoexpand(engine_view.store, engine_view.camera);

                        self.state = TypewriterState::Modifying {
                            modify_state: ModifyState::Up,
                            stroke_key: *stroke_key,
                            cursor: cursor.clone(),
                            pen_down: false,
                        };

                        widget_flags.merge(engine_view.store.record(Instant::now()));
                        widget_flags.resize = true;
                        widget_flags.store_modified = true;
                    }
                    ModifyState::AdjustTextWidth { .. } => {
                        engine_view
                            .store
                            .update_geometry_for_strokes(&[*stroke_key]);
                        engine_view.store.regenerate_rendering_for_stroke(
                            *stroke_key,
                            engine_view.camera.viewport(),
                            engine_view.camera.image_scale(),
                        );
                        engine_view
                            .doc
                            .resize_autoexpand(engine_view.store, engine_view.camera);

                        self.state = TypewriterState::Modifying {
                            modify_state: ModifyState::Up,
                            stroke_key: *stroke_key,
                            cursor: cursor.clone(),
                            pen_down: false,
                        };

                        widget_flags.merge(engine_view.store.record(Instant::now()));
                        widget_flags.resize = true;
                        widget_flags.store_modified = true;
                    }
                }
                PenProgress::InProgress
            }
        };

        (pen_progress, widget_flags)
    }

    pub(super) fn handle_pen_event_proximity(
        &mut self,
        element: Element,
        _modifier_keys: Vec<ModifierKey>,
        _now: Instant,
        engine_view: &mut EngineViewMut,
    ) -> (PenProgress, WidgetFlags) {
        let widget_flags = WidgetFlags::default();
        let typewriter_bounds = self.bounds_on_doc(&engine_view.as_im());

        let pen_progress = match &mut self.state {
            TypewriterState::Idle => PenProgress::Idle,
            TypewriterState::Start(_) => PenProgress::InProgress,
            TypewriterState::Modifying {
                modify_state,
                pen_down,
                ..
            } => {
                // detect hover state
                *modify_state = if typewriter_bounds
                    .map(|b| b.contains_local_point(&na::Point2::from(element.pos)))
                    .unwrap_or(false)
                {
                    ModifyState::Hover(element.pos)
                } else {
                    ModifyState::Up
                };
                *pen_down = false;

                PenProgress::InProgress
            }
        };

        (pen_progress, widget_flags)
    }

    pub(super) fn handle_pen_event_keypressed(
        &mut self,
        keyboard_key: KeyboardKey,
        modifier_keys: Vec<ModifierKey>,
        _now: Instant,
        engine_view: &mut EngineViewMut,
    ) -> (PenProgress, WidgetFlags) {
        let mut widget_flags = WidgetFlags::default();

        let text_width = engine_view.pens_config.typewriter_config.text_width;
        let mut text_style = engine_view.pens_config.typewriter_config.text_style.clone();
        let max_width_enabled = engine_view.pens_config.typewriter_config.max_width_enabled;

        let pen_progress = match &mut self.state {
            TypewriterState::Idle => PenProgress::Idle,
            TypewriterState::Start(pos) => {
                Self::start_audio(Some(keyboard_key), engine_view.audioplayer);

                match keyboard_key {
                    KeyboardKey::Unicode(keychar) => {
                        text_style.ranged_text_attributes.clear();
                        if max_width_enabled {
                            text_style.max_width = Some(text_width);
                        }
                        let textstroke = TextStroke::new(String::from(keychar), *pos, text_style);
                        let mut cursor = GraphemeCursor::new(0, textstroke.text.len(), true);

                        textstroke.move_cursor_forward(&mut cursor);
                        let stroke_key = engine_view
                            .store
                            .insert_stroke(Stroke::TextStroke(textstroke), None);
                        engine_view.store.regenerate_rendering_for_stroke(
                            stroke_key,
                            engine_view.camera.viewport(),
                            engine_view.camera.image_scale(),
                        );

                        self.state = TypewriterState::Modifying {
                            modify_state: ModifyState::Up,
                            stroke_key,
                            cursor,
                            pen_down: false,
                        };

                        widget_flags.merge(engine_view.store.record(Instant::now()));
                        widget_flags.resize = true;
                        widget_flags.store_modified = true;
                    }
                    _ => {}
                }

                PenProgress::InProgress
            }
            TypewriterState::Modifying {
                modify_state,
                stroke_key,
                cursor,
                pen_down,
            } => {
                match modify_state {
                    ModifyState::Up | ModifyState::Hover(_) => {
                        Self::start_audio(Some(keyboard_key), engine_view.audioplayer);

                        if let Some(Stroke::TextStroke(ref mut textstroke)) =
                            engine_view.store.get_stroke_mut(*stroke_key)
                        {
                            let mut update_stroke =
                                |store: &mut StrokeStore, keychar_is_whitespace: bool| {
                                    store.update_geometry_for_stroke(*stroke_key);
                                    store.regenerate_rendering_for_stroke(
                                        *stroke_key,
                                        engine_view.camera.viewport(),
                                        engine_view.camera.image_scale(),
                                    );
                                    engine_view.doc.resize_autoexpand(store, engine_view.camera);

                                    if keychar_is_whitespace {
                                        widget_flags.merge(store.record(Instant::now()));
                                    } else {
                                        widget_flags.merge(
                                            store.update_latest_history_entry(Instant::now()),
                                        );
                                    }

                                    widget_flags.resize = true;
                                    widget_flags.store_modified = true;
                                };

                            // Handling keyboard input
                            match keyboard_key {
                                KeyboardKey::Unicode(keychar) => {
                                    if keychar == 'a'
                                        && modifier_keys.contains(&ModifierKey::KeyboardCtrl)
                                    {
                                        cursor.set_cursor(textstroke.text.len());
                                        // Select entire text
                                        *modify_state = ModifyState::Selecting {
                                            selection_cursor: GraphemeCursor::new(
                                                0,
                                                textstroke.text.len(),
                                                true,
                                            ),
                                            finished: true,
                                        };
                                    } else {
                                        textstroke.insert_text_after_cursor(
                                            keychar.to_string().as_str(),
                                            cursor,
                                        );
                                        update_stroke(engine_view.store, keychar.is_whitespace());
                                    }
                                }
                                KeyboardKey::BackSpace => {
                                    if modifier_keys.contains(&ModifierKey::KeyboardCtrl) {
                                        textstroke.remove_word_before_cursor(cursor);
                                    } else {
                                        textstroke.remove_grapheme_before_cursor(cursor);
                                    }
                                    update_stroke(engine_view.store, false);
                                }
                                KeyboardKey::HorizontalTab => {
                                    textstroke.insert_text_after_cursor("\t", cursor);
                                    update_stroke(engine_view.store, false);
                                }
                                KeyboardKey::CarriageReturn | KeyboardKey::Linefeed => {
                                    textstroke.insert_text_after_cursor("\n", cursor);
                                    update_stroke(engine_view.store, true);
                                }
                                KeyboardKey::Delete => {
                                    if modifier_keys.contains(&ModifierKey::KeyboardCtrl) {
                                        textstroke.remove_word_after_cursor(cursor);
                                    } else {
                                        textstroke.remove_grapheme_after_cursor(cursor);
                                    }
                                    update_stroke(engine_view.store, false);
                                }
                                KeyboardKey::NavLeft => {
                                    if modifier_keys.contains(&ModifierKey::KeyboardShift) {
                                        let old_cursor = cursor.clone();
                                        if modifier_keys.contains(&ModifierKey::KeyboardCtrl) {
                                            textstroke.move_cursor_word_back(cursor);
                                        } else {
                                            textstroke.move_cursor_back(cursor);
                                        }

                                        *modify_state = ModifyState::Selecting {
                                            selection_cursor: old_cursor,
                                            finished: false,
                                        }
                                    } else {
                                        #[allow(clippy::collapsible_else_if)]
                                        if modifier_keys.contains(&ModifierKey::KeyboardCtrl) {
                                            textstroke.move_cursor_word_back(cursor);
                                        } else {
                                            textstroke.move_cursor_back(cursor);
                                        }
                                    }
                                }
                                KeyboardKey::NavRight => {
                                    if modifier_keys.contains(&ModifierKey::KeyboardShift) {
                                        let old_cursor = cursor.clone();
                                        if modifier_keys.contains(&ModifierKey::KeyboardCtrl) {
                                            textstroke.move_cursor_word_forward(cursor);
                                        } else {
                                            textstroke.move_cursor_forward(cursor);
                                        }

                                        *modify_state = ModifyState::Selecting {
                                            selection_cursor: old_cursor,
                                            finished: false,
                                        };
                                    } else {
                                        #[allow(clippy::collapsible_else_if)]
                                        if modifier_keys.contains(&ModifierKey::KeyboardCtrl) {
                                            textstroke.move_cursor_word_forward(cursor);
                                        } else {
                                            textstroke.move_cursor_forward(cursor);
                                        }
                                    }
                                }
                                KeyboardKey::NavUp => {
                                    if modifier_keys.contains(&ModifierKey::KeyboardShift) {
                                        let old_cursor = cursor.clone();
                                        textstroke.move_cursor_line_up(cursor);

                                        *modify_state = ModifyState::Selecting {
                                            selection_cursor: old_cursor,
                                            finished: false,
                                        };
                                    } else {
                                        textstroke.move_cursor_line_up(cursor);
                                    }
                                }
                                KeyboardKey::NavDown => {
                                    if modifier_keys.contains(&ModifierKey::KeyboardShift) {
                                        let old_cursor = cursor.clone();
                                        textstroke.move_cursor_line_down(cursor);

                                        *modify_state = ModifyState::Selecting {
                                            selection_cursor: old_cursor,
                                            finished: false,
                                        };
                                    } else {
                                        textstroke.move_cursor_line_down(cursor);
                                    }
                                }
                                KeyboardKey::Home => {
                                    if modifier_keys.contains(&ModifierKey::KeyboardShift) {
                                        let old_cursor = cursor.clone();
                                        if modifier_keys.contains(&ModifierKey::KeyboardCtrl) {
                                            textstroke.move_cursor_text_start(cursor);
                                        } else {
                                            textstroke.move_cursor_line_start(cursor);
                                        }

                                        *modify_state = ModifyState::Selecting {
                                            selection_cursor: old_cursor,
                                            finished: false,
                                        };
                                    } else {
                                        #[allow(clippy::collapsible_else_if)]
                                        if modifier_keys.contains(&ModifierKey::KeyboardCtrl) {
                                            textstroke.move_cursor_text_start(cursor);
                                        } else {
                                            textstroke.move_cursor_line_start(cursor);
                                        }
                                    }
                                }
                                KeyboardKey::End => {
                                    if modifier_keys.contains(&ModifierKey::KeyboardShift) {
                                        let old_cursor = cursor.clone();
                                        if modifier_keys.contains(&ModifierKey::KeyboardCtrl) {
                                            textstroke.move_cursor_text_end(cursor);
                                        } else {
                                            textstroke.move_cursor_line_end(cursor);
                                        }

                                        *modify_state = ModifyState::Selecting {
                                            selection_cursor: old_cursor,
                                            finished: false,
                                        };
                                    } else {
                                        #[allow(clippy::collapsible_else_if)]
                                        if modifier_keys.contains(&ModifierKey::KeyboardCtrl) {
                                            textstroke.move_cursor_text_end(cursor);
                                        } else {
                                            textstroke.move_cursor_line_end(cursor);
                                        }
                                    }
                                }
                                _ => {}
                            };

                            *pen_down = false;
                        }

                        PenProgress::InProgress
                    }
                    ModifyState::Selecting {
                        selection_cursor,
                        finished,
                    } => {
                        Self::start_audio(Some(keyboard_key), engine_view.audioplayer);

                        if let Some(Stroke::TextStroke(textstroke)) =
                            engine_view.store.get_stroke_mut(*stroke_key)
                        {
                            let mut update_stroke = |store: &mut StrokeStore| {
                                store.update_geometry_for_stroke(*stroke_key);
                                store.regenerate_rendering_for_stroke(
                                    *stroke_key,
                                    engine_view.camera.viewport(),
                                    engine_view.camera.image_scale(),
                                );
                                engine_view.doc.resize_autoexpand(store, engine_view.camera);

                                widget_flags.merge(store.record(Instant::now()));
                                widget_flags.resize = true;
                                widget_flags.store_modified = true;
                            };

                            // Handle keyboard keys
                            let quit_selecting = match keyboard_key {
                                KeyboardKey::Unicode(keychar) => {
                                    if keychar == 'a'
                                        && modifier_keys.contains(&ModifierKey::KeyboardCtrl)
                                    {
                                        textstroke
                                            .update_selection_entire_text(cursor, selection_cursor);
                                        *finished = true;
                                        false
                                    } else {
                                        textstroke.replace_text_between_selection_cursors(
                                            cursor,
                                            selection_cursor,
                                            String::from(keychar).as_str(),
                                        );
                                        update_stroke(engine_view.store);
                                        true
                                    }
                                }
                                KeyboardKey::NavLeft => {
                                    if modifier_keys.contains(&ModifierKey::KeyboardShift) {
                                        if modifier_keys.contains(&ModifierKey::KeyboardCtrl) {
                                            textstroke.move_cursor_word_back(cursor);
                                        } else {
                                            textstroke.move_cursor_back(cursor);
                                        }
                                        false
                                    } else {
                                        cursor.set_cursor(
                                            cursor.cur_cursor().min(selection_cursor.cur_cursor()),
                                        );
                                        true
                                    }
                                }
                                KeyboardKey::NavRight => {
                                    if modifier_keys.contains(&ModifierKey::KeyboardShift) {
                                        if modifier_keys.contains(&ModifierKey::KeyboardCtrl) {
                                            textstroke.move_cursor_word_forward(cursor);
                                        } else {
                                            textstroke.move_cursor_forward(cursor);
                                        }
                                        false
                                    } else {
                                        cursor.set_cursor(
                                            cursor.cur_cursor().max(selection_cursor.cur_cursor()),
                                        );
                                        true
                                    }
                                }
                                KeyboardKey::NavUp => {
                                    textstroke.move_cursor_line_up(cursor);
                                    !modifier_keys.contains(&ModifierKey::KeyboardShift)
                                }
                                KeyboardKey::NavDown => {
                                    textstroke.move_cursor_line_down(cursor);
                                    !modifier_keys.contains(&ModifierKey::KeyboardShift)
                                }
                                KeyboardKey::Home => {
                                    if modifier_keys.contains(&ModifierKey::KeyboardCtrl) {
                                        textstroke.move_cursor_text_start(cursor);
                                    } else {
                                        textstroke.move_cursor_line_start(cursor);
                                    }
                                    !modifier_keys.contains(&ModifierKey::KeyboardShift)
                                }
                                KeyboardKey::End => {
                                    if modifier_keys.contains(&ModifierKey::KeyboardCtrl) {
                                        textstroke.move_cursor_text_end(cursor);
                                    } else {
                                        textstroke.move_cursor_line_end(cursor);
                                    }
                                    !modifier_keys.contains(&ModifierKey::KeyboardShift)
                                }
                                KeyboardKey::CarriageReturn | KeyboardKey::Linefeed => {
                                    textstroke.replace_text_between_selection_cursors(
                                        cursor,
                                        selection_cursor,
                                        "\n",
                                    );
                                    update_stroke(engine_view.store);
                                    true
                                }
                                KeyboardKey::BackSpace | KeyboardKey::Delete => {
                                    textstroke.replace_text_between_selection_cursors(
                                        cursor,
                                        selection_cursor,
                                        "",
                                    );
                                    update_stroke(engine_view.store);
                                    true
                                }
                                KeyboardKey::HorizontalTab => {
                                    textstroke.replace_text_between_selection_cursors(
                                        cursor,
                                        selection_cursor,
                                        "\t",
                                    );
                                    update_stroke(engine_view.store);
                                    true
                                }
                                KeyboardKey::CtrlLeft
                                | KeyboardKey::CtrlRight
                                | KeyboardKey::ShiftLeft
                                | KeyboardKey::ShiftRight => false,
                                _ => true,
                            };

                            if quit_selecting {
                                // Back to modifying
                                self.state = TypewriterState::Modifying {
                                    modify_state: ModifyState::Up,
                                    stroke_key: *stroke_key,
                                    cursor: cursor.clone(),
                                    pen_down: false,
                                };
                            }
                        }

                        PenProgress::InProgress
                    }
                    _ => PenProgress::InProgress,
                }
            }
        };

        (pen_progress, widget_flags)
    }

    pub(super) fn handle_pen_event_text(
        &mut self,
        text: String,
        _now: Instant,
        engine_view: &mut EngineViewMut,
    ) -> (PenProgress, WidgetFlags) {
        let mut widget_flags = WidgetFlags::default();

        let text_width = engine_view.pens_config.typewriter_config.text_width;
        let mut text_style = engine_view.pens_config.typewriter_config.text_style.clone();
        let max_width_enabled = engine_view.pens_config.typewriter_config.max_width_enabled;

        let pen_progress = match &mut self.state {
            TypewriterState::Idle => PenProgress::Idle,
            TypewriterState::Start(pos) => {
                Self::start_audio(None, engine_view.audioplayer);

                text_style.ranged_text_attributes.clear();
                if max_width_enabled {
                    text_style.max_width = Some(text_width);
                }
                let text_len = text.len();
                let textstroke = TextStroke::new(text, *pos, text_style);
                let cursor = GraphemeCursor::new(text_len, text_len, true);

                let stroke_key = engine_view
                    .store
                    .insert_stroke(Stroke::TextStroke(textstroke), None);
                engine_view.store.regenerate_rendering_for_stroke(
                    stroke_key,
                    engine_view.camera.viewport(),
                    engine_view.camera.image_scale(),
                );

                self.state = TypewriterState::Modifying {
                    modify_state: ModifyState::Up,
                    stroke_key,
                    cursor,
                    pen_down: false,
                };

                widget_flags.merge(engine_view.store.record(Instant::now()));
                widget_flags.resize = true;
                widget_flags.store_modified = true;

                PenProgress::InProgress
            }
            TypewriterState::Modifying {
                modify_state,
                stroke_key,
                cursor,
                pen_down,
            } => {
                match modify_state {
                    ModifyState::Up | ModifyState::Hover(_) => {
                        Self::start_audio(None, engine_view.audioplayer);

                        if let Some(Stroke::TextStroke(ref mut textstroke)) =
                            engine_view.store.get_stroke_mut(*stroke_key)
                        {
                            textstroke.insert_text_after_cursor(&text, cursor);
                            engine_view.store.update_geometry_for_stroke(*stroke_key);
                            engine_view.store.regenerate_rendering_for_stroke(
                                *stroke_key,
                                engine_view.camera.viewport(),
                                engine_view.camera.image_scale(),
                            );
                            engine_view
                                .doc
                                .resize_autoexpand(engine_view.store, engine_view.camera);

                            *pen_down = false;

                            // only record new history entry if the text contains ascii-whitespace,
                            // else only update history
                            if text.contains(char::is_whitespace) {
                                widget_flags.merge(engine_view.store.record(Instant::now()));
                            } else {
                                widget_flags.merge(
                                    engine_view
                                        .store
                                        .update_latest_history_entry(Instant::now()),
                                );
                            }
                            widget_flags.resize = true;
                            widget_flags.store_modified = true;
                        }

                        PenProgress::InProgress
                    }
                    ModifyState::Selecting {
                        selection_cursor,
                        finished,
                    } => {
                        Self::start_audio(None, engine_view.audioplayer);

                        if let Some(Stroke::TextStroke(textstroke)) =
                            engine_view.store.get_stroke_mut(*stroke_key)
                        {
                            textstroke.replace_text_between_selection_cursors(
                                cursor,
                                selection_cursor,
                                text.as_str(),
                            );
                            engine_view.store.update_geometry_for_stroke(*stroke_key);
                            engine_view.store.regenerate_rendering_for_stroke(
                                *stroke_key,
                                engine_view.camera.viewport(),
                                engine_view.camera.image_scale(),
                            );
                            engine_view
                                .doc
                                .resize_autoexpand(engine_view.store, engine_view.camera);

                            *finished = true;

                            // only record new history entry if the text contains ascii-whitespace,
                            // else only update history
                            if text.contains(char::is_whitespace) {
                                widget_flags.merge(engine_view.store.record(Instant::now()));
                            } else {
                                widget_flags.merge(
                                    engine_view
                                        .store
                                        .update_latest_history_entry(Instant::now()),
                                );
                            }
                            widget_flags.resize = true;
                            widget_flags.store_modified = true;
                        }

                        PenProgress::InProgress
                    }
                    _ => PenProgress::InProgress,
                }
            }
        };

        (pen_progress, widget_flags)
    }

    pub(super) fn handle_pen_event_cancel(
        &mut self,
        _now: Instant,
        _engine_view: &mut EngineViewMut,
    ) -> (PenProgress, WidgetFlags) {
        let widget_flags = WidgetFlags::default();

        let pen_progress = match &mut self.state {
            TypewriterState::Idle => PenProgress::Idle,
            _ => {
                self.state = TypewriterState::Idle;

                PenProgress::Finished
            }
        };

        (pen_progress, widget_flags)
    }
}
