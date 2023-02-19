use std::time::Instant;

use crate::engine::{EngineView, EngineViewMut};
use crate::pens::shortcuts::ShortcutAction;

use crate::widgetflags::WidgetFlags;
use crate::DrawOnDocBehaviour;
use piet::RenderContext;
use rnote_compose::penevents::{PenEvent, ShortcutKey};

use p2d::bounding_volume::Aabb;
use serde::{Deserialize, Serialize};

use super::penbehaviour::PenProgress;
use super::penmode::PenModeState;
use super::shortcuts::ShortcutMode;
use super::{
    Brush, Eraser, Pen, PenBehaviour, PenMode, PenStyle, Selector, Shaper, Shortcuts, Tools,
    Typewriter,
};

/// This holds the pens and related state and handles pen events.
#[allow(missing_debug_implementations)]
#[derive(Serialize, Deserialize)]
#[serde(default, rename = "penholder")]
pub struct PenHolder {
    #[serde(rename = "shortcuts")]
    pub shortcuts: Shortcuts,
    #[serde(rename = "pen_mode_state")]
    pub pen_mode_state: PenModeState,

    #[serde(skip)]
    pub(super) current_pen: Pen,
    #[serde(skip)]
    pen_progress: PenProgress,
    #[serde(skip)]
    toggle_pen_style: Option<PenStyle>,
    #[serde(skip)]
    prev_shortcut_key: Option<ShortcutKey>,
}

impl Default for PenHolder {
    fn default() -> Self {
        Self {
            shortcuts: Shortcuts::default(),
            pen_mode_state: PenModeState::default(),

            current_pen: Pen::default(),
            pen_progress: PenProgress::Idle,
            toggle_pen_style: None,
            prev_shortcut_key: None,
        }
    }
}

impl PenHolder {
    pub fn clear_shortcuts(&mut self) {
        self.shortcuts.clear();
    }
    /// Registers a new shortcut key and action
    pub fn register_new_shortcut(&mut self, key: ShortcutKey, action: ShortcutAction) {
        self.shortcuts.insert(key, action);
    }

    /// Removes the shortcut action for the given shortcut key, if it is registered
    pub fn remove_shortcut(&mut self, key: ShortcutKey) -> Option<ShortcutAction> {
        self.shortcuts.remove(&key)
    }

    // Gets the current registered action the the given shortcut key
    pub fn get_shortcut_action(&self, key: ShortcutKey) -> Option<ShortcutAction> {
        self.shortcuts.get(&key).cloned()
    }

    /// Lists all current registered shortcut keys and their action
    pub fn list_current_shortcuts(&self) -> Vec<(ShortcutKey, ShortcutAction)> {
        self.shortcuts
            .iter()
            .map(|(key, action)| (*key, *action))
            .collect()
    }

    /// Gets the style without the temporary override.
    pub fn current_pen_style(&self) -> PenStyle {
        self.pen_mode_state.style()
    }

    /// Gets the current style, or the override if it is set.
    pub fn current_pen_style_w_override(&self) -> PenStyle {
        self.pen_mode_state.current_style_w_override()
    }

    /// the current pen progress
    pub fn current_pen_progress(&self) -> PenProgress {
        self.pen_progress
    }

    pub fn current_pen_ref(&mut self) -> &Pen {
        &self.current_pen
    }

    pub fn current_pen_mut(&mut self) -> &mut Pen {
        &mut self.current_pen
    }

    /// change the pen style
    pub fn change_style(
        &mut self,
        new_style: PenStyle,
        engine_view: &mut EngineViewMut,
    ) -> WidgetFlags {
        let widget_flags = self.change_style_int(new_style, engine_view);
        // When the style is changed externally, the toggle mode / internal states are reset
        self.toggle_pen_style = None;
        self.prev_shortcut_key = None;

        widget_flags
    }

    /// change the style override
    pub fn change_style_override(
        &mut self,
        new_style_override: Option<PenStyle>,
        engine_view: &mut EngineViewMut,
    ) -> WidgetFlags {
        let mut widget_flags = WidgetFlags::default();

        //log::debug!("current_style_override: {:?}, new_style_override: {:?}", self.style_override, new_style_override);

        if self.pen_mode_state.style_override() != new_style_override {
            // Deselecting when changing the style override
            let all_strokes = engine_view.store.selection_keys_as_rendered();
            engine_view.store.set_selected_keys(&all_strokes, false);

            self.pen_mode_state.set_style_override(new_style_override);

            widget_flags.merge(self.reinstall_pen_current_style(engine_view));
        }

        widget_flags
    }

    /// change the pen mode (pen, eraser, etc.). Relevant for stylus input
    pub fn change_pen_mode(
        &mut self,
        new_pen_mode: PenMode,
        engine_view: &mut EngineViewMut,
    ) -> WidgetFlags {
        let mut widget_flags = WidgetFlags::default();

        if self.pen_mode_state.pen_mode() != new_pen_mode {
            self.pen_mode_state.set_pen_mode(new_pen_mode);

            widget_flags.merge(self.reinstall_pen_current_style(engine_view));
        }

        widget_flags
    }

    pub fn update_state_current_pen(&mut self, engine_view: &mut EngineViewMut) -> WidgetFlags {
        self.current_pen.update_state(engine_view)
    }

    /// Installs the pen for the current style
    pub fn reinstall_pen_current_style(&mut self, engine_view: &mut EngineViewMut) -> WidgetFlags {
        let (new_pen, mut widget_flags) =
            new_pen_for_style(self.current_pen_style_w_override(), engine_view);
        self.current_pen = new_pen;
        widget_flags.merge(self.current_pen.update_state(engine_view));
        widget_flags.merge(self.handle_changed_pen_style());
        self.pen_progress = PenProgress::Idle;

        widget_flags
    }

    /// Handle a pen event
    pub fn handle_pen_event(
        &mut self,
        event: PenEvent,
        pen_mode: Option<PenMode>,
        now: Instant,
        engine_view: &mut EngineViewMut,
    ) -> WidgetFlags {
        /*
               log::debug!(
                   "handle_pen_event(), event: {:?}, pen_mode_state: {:?}",
                   event,
                   self.pen_mode_state,
               );
        */
        let mut widget_flags = WidgetFlags::default();
        widget_flags.redraw = true;

        if let Some(pen_mode) = pen_mode {
            widget_flags.merge(self.change_pen_mode(pen_mode, engine_view));
        }

        // Handle the event with the current pen
        let (pen_progress, other_widget_flags) =
            self.current_pen.handle_event(event, now, engine_view);

        widget_flags.merge(other_widget_flags);

        widget_flags.merge(self.handle_pen_progress(pen_progress, engine_view));

        widget_flags
    }

    fn change_style_int(
        &mut self,
        new_style: PenStyle,
        engine_view: &mut EngineViewMut,
    ) -> WidgetFlags {
        let mut widget_flags = WidgetFlags::default();

        if self.pen_mode_state.style() != new_style {
            // Deselecting when changing the style
            let all_strokes = engine_view.store.selection_keys_as_rendered();
            engine_view.store.set_selected_keys(&all_strokes, false);

            self.pen_mode_state.set_style(new_style);
            widget_flags.merge(self.reinstall_pen_current_style(engine_view));
        }

        widget_flags
    }

    fn handle_pen_progress(
        &mut self,
        pen_progress: PenProgress,
        engine_view: &mut EngineViewMut,
    ) -> WidgetFlags {
        let mut widget_flags = WidgetFlags::default();

        match pen_progress {
            PenProgress::Idle => {}
            PenProgress::InProgress => {}
            PenProgress::Finished => {
                // take the style override when pen is finished
                if self.pen_mode_state.take_style_override().is_some() {
                    widget_flags.refresh_ui = true;
                }

                widget_flags.merge(self.reinstall_pen_current_style(engine_view));
            }
        }

        self.pen_progress = pen_progress;

        widget_flags
    }

    fn handle_changed_pen_style(&mut self) -> WidgetFlags {
        let mut widget_flags = WidgetFlags::default();

        match self.pen_mode_state.current_style_w_override() {
            PenStyle::Typewriter => {
                // Enable text preprocessing for typewriter
                widget_flags.enable_text_preprocessing = Some(true);
            }
            _ => {
                widget_flags.enable_text_preprocessing = Some(false);
            }
        }
        widget_flags.redraw = true;
        widget_flags.refresh_ui = true;

        widget_flags
    }

    /// Handle a pressed shortcut key
    pub fn handle_pressed_shortcut_key(
        &mut self,
        shortcut_key: ShortcutKey,
        _now: Instant,
        engine_view: &mut EngineViewMut,
    ) -> WidgetFlags {
        let mut widget_flags = WidgetFlags::default();

        if let Some(action) = self.get_shortcut_action(shortcut_key) {
            match action {
                ShortcutAction::ChangePenStyle { style, mode } => match mode {
                    ShortcutMode::Temporary => {
                        widget_flags.merge(self.change_style_override(Some(style), engine_view));
                    }
                    ShortcutMode::Permanent => {
                        self.toggle_pen_style = None;
                        widget_flags.merge(self.change_style_int(style, engine_view));
                    }
                    ShortcutMode::Toggle => {
                        if let Some(toggle_pen_style) = self.toggle_pen_style {
                            // if the previous key was different, but also in toggle mode, we switch to the new style instead of toggling back
                            if self
                                .prev_shortcut_key
                                .map(|k| k != shortcut_key)
                                .unwrap_or(true)
                            {
                                widget_flags.merge(self.change_style_int(style, engine_view));
                            } else {
                                self.toggle_pen_style = None;
                                widget_flags
                                    .merge(self.change_style_int(toggle_pen_style, engine_view));
                            }
                        } else {
                            self.toggle_pen_style = Some(self.current_pen_style());
                            widget_flags.merge(self.change_style_int(style, engine_view));
                        }
                    }
                },
            }
        }

        self.prev_shortcut_key = Some(shortcut_key);

        widget_flags
    }

    /// fetches clipboard content from the current pen
    #[allow(clippy::type_complexity)]
    pub fn fetch_clipboard_content(
        &self,
        engine_view: &EngineView,
    ) -> anyhow::Result<(Option<(Vec<u8>, String)>, WidgetFlags)> {
        self.current_pen.fetch_clipboard_content(engine_view)
    }

    /// cuts clipboard content from the current pen
    #[allow(clippy::type_complexity)]
    pub fn cut_clipboard_content(
        &mut self,
        engine_view: &mut EngineViewMut,
    ) -> anyhow::Result<(Option<(Vec<u8>, String)>, WidgetFlags)> {
        self.current_pen.cut_clipboard_content(engine_view)
    }
}

impl DrawOnDocBehaviour for PenHolder {
    fn bounds_on_doc(&self, engine_view: &EngineView) -> Option<Aabb> {
        self.current_pen.bounds_on_doc(engine_view)
    }
    fn draw_on_doc(
        &self,
        cx: &mut piet_cairo::CairoRenderContext,
        engine_view: &EngineView,
    ) -> anyhow::Result<()> {
        cx.save().map_err(|e| anyhow::anyhow!("{e:?}"))?;

        self.current_pen.draw_on_doc(cx, engine_view)?;

        cx.restore().map_err(|e| anyhow::anyhow!("{e:?}"))?;
        Ok(())
    }
}

fn new_pen_for_style(pen_style: PenStyle, engine_view: &mut EngineViewMut) -> (Pen, WidgetFlags) {
    let mut pen = match pen_style {
        PenStyle::Brush => Pen::Brush(Brush::default()),
        PenStyle::Shaper => Pen::Shaper(Shaper::default()),
        PenStyle::Typewriter => Pen::Typewriter(Typewriter::default()),
        PenStyle::Eraser => Pen::Eraser(Eraser::default()),
        PenStyle::Selector => Pen::Selector(Selector::default()),
        PenStyle::Tools => Pen::Tools(Tools::default()),
    };

    let widget_flags = pen.update_state(engine_view);

    (pen, widget_flags)
}
