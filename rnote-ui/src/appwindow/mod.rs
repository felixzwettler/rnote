mod appsettings;
mod appwindowactions;
mod imp;

use adw::{prelude::*, subclass::prelude::*};
use gettextrs::gettext;
use gtk4::gdk;
use gtk4::{gio, glib, glib::clone, Application, Box, Button, FileChooserNative, IconTheme};
use rnote_compose::Color;
use rnote_engine::pens::pensconfig::brushconfig::BrushStyle;
use rnote_engine::pens::pensconfig::shaperconfig::ShaperStyle;
use rnote_engine::pens::PenStyle;
use rnote_engine::utils::GdkRGBAHelpers;
use rnote_engine::{engine::EngineTask, WidgetFlags};
use std::cell::RefCell;
use std::path::Path;
use std::rc::Rc;

use crate::canvas::RnoteCanvas;
use crate::{
    config, RnoteApp, RnoteCanvasWrapper, RnoteOverlays, SettingsPanel, WorkspaceBrowser,
    {dialogs, MainHeader},
};

glib::wrapper! {
    pub(crate) struct RnoteAppWindow(ObjectSubclass<imp::RnoteAppWindow>)
        @extends gtk4::Widget, gtk4::Window, adw::Window, gtk4::ApplicationWindow, adw::ApplicationWindow,
        @implements gio::ActionMap, gio::ActionGroup;
}

impl RnoteAppWindow {
    const AUTOSAVE_INTERVAL_DEFAULT: u32 = 30;
    const PERIODIC_CONFIGSAVE_INTERVAL: u32 = 10;
    const FLAP_FOLDED_RESIZE_MARGIN: u32 = 64;

    pub(crate) fn new(app: &Application) -> Self {
        glib::Object::new(&[("application", app)])
    }

    #[allow(unused)]
    pub(crate) fn autosave(&self) -> bool {
        self.property::<bool>("autosave")
    }

    #[allow(unused)]
    pub(crate) fn set_autosave(&self, autosave: bool) {
        self.set_property("autosave", autosave.to_value());
    }

    #[allow(unused)]
    pub(crate) fn autosave_interval_secs(&self) -> u32 {
        self.property::<u32>("autosave-interval-secs")
    }

    #[allow(unused)]
    pub(crate) fn set_autosave_interval_secs(&self, autosave_interval_secs: u32) {
        self.set_property("autosave-interval-secs", autosave_interval_secs.to_value());
    }

    #[allow(unused)]
    pub(crate) fn righthanded(&self) -> bool {
        self.property::<bool>("righthanded")
    }

    #[allow(unused)]
    pub(crate) fn set_righthanded(&self, righthanded: bool) {
        self.set_property("righthanded", righthanded.to_value());
    }

    #[allow(unused)]
    pub(crate) fn touch_drawing(&self) -> bool {
        self.property::<bool>("touch-drawing")
    }

    #[allow(unused)]
    pub(crate) fn set_touch_drawing(&self, touch_drawing: bool) {
        self.set_property("touch-drawing", touch_drawing.to_value());
    }

    pub(crate) fn app(&self) -> RnoteApp {
        self.application().unwrap().downcast::<RnoteApp>().unwrap()
    }

    pub(crate) fn app_settings(&self) -> gio::Settings {
        self.imp().app_settings.clone()
    }

    pub(crate) fn filechoosernative(&self) -> Rc<RefCell<Option<FileChooserNative>>> {
        self.imp().filechoosernative.clone()
    }

    pub(crate) fn overlays(&self) -> RnoteOverlays {
        self.imp().overlays.get()
    }

    pub(crate) fn settings_panel(&self) -> SettingsPanel {
        self.imp().settings_panel.get()
    }

    pub(crate) fn flap_box(&self) -> gtk4::Box {
        self.imp().flap_box.get()
    }

    pub(crate) fn flap_header(&self) -> adw::HeaderBar {
        self.imp().flap_header.get()
    }

    pub(crate) fn workspacebrowser(&self) -> WorkspaceBrowser {
        self.imp().workspacebrowser.get()
    }

    pub(crate) fn flap(&self) -> adw::Flap {
        self.imp().flap.get()
    }

    pub(crate) fn flap_menus_box(&self) -> Box {
        self.imp().flap_menus_box.get()
    }

    pub(crate) fn flap_close_button(&self) -> Button {
        self.imp().flap_close_button.get()
    }

    pub(crate) fn flap_stack(&self) -> adw::ViewStack {
        self.imp().flap_stack.get()
    }

    pub(crate) fn mainheader(&self) -> MainHeader {
        self.imp().mainheader.get()
    }

    // Must be called after application is associated with it else it fails
    pub(crate) fn init(&self) {
        let imp = self.imp();

        imp.overlays.get().init(self);
        imp.workspacebrowser.get().init(self);
        imp.settings_panel.get().init(self);
        imp.mainheader.get().init(self);
        imp.mainheader.get().canvasmenu().init(self);
        imp.mainheader.get().appmenu().init(self);

        // An initial tab. Must! come before setting up the settings binds and import
        self.add_initial_tab();

        // add icon theme resource path because automatic lookup does not work in the devel build.
        let app_icon_theme = IconTheme::for_display(&self.display());
        app_icon_theme.add_resource_path((String::from(config::APP_IDPATH) + "icons").as_str());

        // actions and settings AFTER widget inits
        self.setup_actions();
        self.setup_action_accels();
        self.setup_settings_binds();
        self.load_settings();

        // Periodically save engine config
        if let Some(removed_id) = self.imp().periodic_configsave_source_id.borrow_mut().replace(
            glib::source::timeout_add_seconds_local(
                Self::PERIODIC_CONFIGSAVE_INTERVAL, clone!(@weak self as appwindow => @default-return glib::source::Continue(false), move || {
                    if let Err(e) = appwindow.active_tab().canvas().save_engine_config(&appwindow.app_settings()) {
                        log::error!("saving engine config in periodic task failed with Err: {e:?}");
                    }

                    glib::source::Continue(true)
        }))) {
            removed_id.remove();
        }

        // Anything that needs to be done right before showing the appwindow

        // Set undo / redo as not sensitive as default ( setting it in .ui file did not work for some reason )
        self.mainheader().undo_button().set_sensitive(false);
        self.mainheader().redo_button().set_sensitive(false);
        self.refresh_ui_from_engine(&self.active_tab());
    }

    /// Called to close the window
    pub(crate) fn close_force(&self) {
        // Saving all state
        if let Err(e) = self.save_to_settings() {
            log::error!("Failed to save appwindow to settings, with Err: {e:?}");
        }

        // Closing the state tasks channel receiver for all tabs
        for tab in self
            .tab_pages_snapshot()
            .into_iter()
            .map(|p| p.child().downcast::<RnoteCanvasWrapper>().unwrap())
        {
            if let Err(e) = tab
                .canvas()
                .engine()
                .borrow()
                .tasks_tx()
                .unbounded_send(EngineTask::Quit)
            {
                log::error!(
                    "failed to send StateTask::Quit to tab with title `{}`, Err: {e:?}",
                    tab.canvas().doc_title_display()
                );
            }
        }

        self.destroy();
    }

    // Returns true if the flags indicate that any loop that handles the flags should be quit. (usually an async event loop)
    pub(crate) fn handle_widget_flags(&self, widget_flags: WidgetFlags, canvas: &RnoteCanvas) {
        if widget_flags.redraw {
            canvas.queue_draw();
        }
        if widget_flags.resize {
            canvas.queue_resize();
        }
        if widget_flags.refresh_ui {
            self.refresh_ui_from_engine(&self.active_tab());
        }
        if widget_flags.store_modified {
            canvas.set_unsaved_changes(true);
            canvas.set_empty(false);
        }
        if widget_flags.update_view {
            let camera_offset = canvas.engine().borrow().camera.offset;
            // this updates the canvas adjustment values with the ones from the camera
            canvas.update_camera_offset(camera_offset);
        }
        if let Some(hide_undo) = widget_flags.hide_undo {
            self.mainheader().undo_button().set_sensitive(!hide_undo);
        }
        if let Some(hide_redo) = widget_flags.hide_redo {
            self.mainheader().redo_button().set_sensitive(!hide_redo);
        }
        if let Some(enable_text_preprocessing) = widget_flags.enable_text_preprocessing {
            canvas.set_text_preprocessing(enable_text_preprocessing);
        }
    }

    /// Get the active (selected) tab page.
    /// Panics if there is none (but should never be the case, since we add one initially and the UI hides closing the last tab)
    pub(crate) fn active_tab_page(&self) -> adw::TabPage {
        self.imp()
            .overlays
            .tabview()
            .selected_page()
            .expect("there must always be one active tab")
    }

    /// Get the active (selected) tab page child.
    pub(crate) fn active_tab(&self) -> RnoteCanvasWrapper {
        self.active_tab_page()
            .child()
            .downcast::<RnoteCanvasWrapper>()
            .unwrap()
    }

    /// adds the initial tab to the tabview
    fn add_initial_tab(&self) -> adw::TabPage {
        let new_wrapper = RnoteCanvasWrapper::new();
        if let Err(e) = new_wrapper
            .canvas()
            .load_engine_config(&self.app_settings())
        {
            log::debug!("failed to load engine config for initial tab, Err: {e:?}");
        }
        self.overlays().tabview().append(&new_wrapper)
    }

    /// Creates a new tab and set it as selected
    pub(crate) fn new_tab(&self) -> adw::TabPage {
        let current_engine_config = match self
            .active_tab()
            .canvas()
            .engine()
            .borrow()
            .extract_engine_config()
        {
            Ok(c) => Some(c),
            Err(e) => {
                log::error!("failed to extract engine config from active tab, Err: {e:?}");
                None
            }
        };
        let new_wrapper = RnoteCanvasWrapper::new();
        if let Some(current_engine_config) = current_engine_config {
            match new_wrapper
                .canvas()
                .engine()
                .borrow_mut()
                .load_engine_config(current_engine_config, Some(config::DATADIR.into()))
            {
                Ok(wf) => self.handle_widget_flags(wf, &new_wrapper.canvas()),
                Err(e) => {
                    log::error!("failed to load current engine config into new tab, Err: {e:?}")
                }
            }
        }

        // The tab page connections are handled in page_attached, which is fired when the page is added to the tabview
        let page = self.overlays().tabview().append(&new_wrapper);
        self.overlays().tabview().set_selected_page(&page);

        page
    }

    pub(crate) fn tab_pages_snapshot(&self) -> Vec<adw::TabPage> {
        self.overlays()
            .tabview()
            .pages()
            .snapshot()
            .into_iter()
            .map(|o| o.downcast::<adw::TabPage>().unwrap())
            .collect()
    }

    pub(crate) fn tabs_any_unsaved_changes(&self) -> bool {
        self.overlays()
            .tabview()
            .pages()
            .snapshot()
            .iter()
            .map(|o| {
                o.downcast_ref::<adw::TabPage>()
                    .unwrap()
                    .child()
                    .downcast_ref::<RnoteCanvasWrapper>()
                    .unwrap()
                    .canvas()
            })
            .any(|c| c.unsaved_changes())
    }

    pub(crate) fn tabs_query_file_opened(
        &self,
        input_file_path: impl AsRef<Path>,
    ) -> Option<adw::TabPage> {
        self.overlays()
            .tabview()
            .pages()
            .snapshot()
            .into_iter()
            .filter_map(|o| {
                let tab_page = o.downcast::<adw::TabPage>().unwrap();
                Some((
                    tab_page.clone(),
                    tab_page
                        .child()
                        .downcast_ref::<RnoteCanvasWrapper>()
                        .unwrap()
                        .canvas()
                        .output_file()?
                        .path()?,
                ))
            })
            .find(|(_, output_file_path)| {
                same_file::is_same_file(output_file_path, input_file_path.as_ref()).unwrap_or(false)
            })
            .map(|(found, _)| found)
    }

    pub(crate) fn clear_rendering_inactive_tabs(&self) {
        for inactive_page in self
            .overlays()
            .tabview()
            .pages()
            .snapshot()
            .into_iter()
            .map(|o| o.downcast::<adw::TabPage>().unwrap())
            .filter(|p| !p.is_selected())
        {
            inactive_page
                .child()
                .downcast::<RnoteCanvasWrapper>()
                .unwrap()
                .canvas()
                .engine()
                .borrow_mut()
                .clear_rendering();
        }
    }

    pub(crate) fn refresh_titles(&self, active_tab: &RnoteCanvasWrapper) {
        let canvas = active_tab.canvas();

        // Titles
        let title = canvas.doc_title_display();
        let subtitle = canvas.doc_folderpath_display();

        self.set_title(Some(
            &(title.clone() + " - " + config::APP_NAME_CAPITALIZED),
        ));

        self.mainheader()
            .main_title_unsaved_indicator()
            .set_visible(canvas.unsaved_changes());
        if canvas.unsaved_changes() {
            self.mainheader()
                .main_title()
                .add_css_class("unsaved_changes");
        } else {
            self.mainheader()
                .main_title()
                .remove_css_class("unsaved_changes");
        }

        self.mainheader().main_title().set_title(&title);
        self.mainheader().main_title().set_subtitle(&subtitle);
    }

    /// Opens the file, with import dialogs when appropriate.
    ///
    /// When the file is a rnote save file, `rnote_file_new_tab` determines if a new tab is opened, or if it overwrites the current active one.
    pub(crate) fn open_file_w_dialogs(
        &self,
        input_file: gio::File,
        target_pos: Option<na::Vector2<f64>>,
        rnote_file_new_tab: bool,
    ) {
        match crate::utils::FileType::lookup_file_type(&input_file) {
            crate::utils::FileType::RnoteFile => {
                let Some(input_file_path) = input_file.path() else {
                    log::error!("could not open file: {input_file:?}, path returned None");
                    return;
                };

                // If the file is already opened in a tab, simply switch to it
                if let Some(page) = self.tabs_query_file_opened(input_file_path) {
                    self.overlays().tabview().set_selected_page(&page);
                } else {
                    let canvas = if rnote_file_new_tab {
                        // open a new tab for rnote files
                        let new_tab = self.new_tab();
                        new_tab
                            .child()
                            .downcast::<RnoteCanvasWrapper>()
                            .unwrap()
                            .canvas()
                    } else {
                        self.active_tab().canvas()
                    };

                    if let Err(e) = self.load_in_file(input_file, target_pos, &canvas) {
                        log::error!(
                        "failed to load in file with FileType::RnoteFile | FileType::XoppFile, {e:?}"
                    );
                    }
                }
            }
            crate::utils::FileType::VectorImageFile | crate::utils::FileType::BitmapImageFile => {
                if let Err(e) =
                    self.load_in_file(input_file, target_pos, &self.active_tab().canvas())
                {
                    log::error!("failed to load in file with FileType::VectorImageFile / FileType::BitmapImageFile / FileType::Pdf, {e:?}");
                }
            }
            crate::utils::FileType::XoppFile => {
                // open a new tab for xopp file import
                let new_tab = self.new_tab();
                let canvas = new_tab
                    .child()
                    .downcast::<RnoteCanvasWrapper>()
                    .unwrap()
                    .canvas();

                dialogs::import::dialog_import_xopp_w_prefs(self, &canvas, input_file);
            }
            crate::utils::FileType::PdfFile => {
                dialogs::import::dialog_import_pdf_w_prefs(
                    self,
                    &self.active_tab().canvas(),
                    input_file,
                    target_pos,
                );
            }
            crate::utils::FileType::Folder => {
                if let Some(dir) = input_file.path() {
                    self.workspacebrowser()
                        .workspacesbar()
                        .set_selected_workspace_dir(dir);
                }
            }
            crate::utils::FileType::Unsupported => {
                log::error!("tried to open unsupported file type.");
            }
        }
    }

    /// Loads in a file of any supported type into the engine of the given canvas.
    ///
    /// ! if the file is a rnote save file, it will overwrite the state in the active tab so there should be a user prompt to confirm before this is called
    pub(crate) fn load_in_file(
        &self,
        file: gio::File,
        target_pos: Option<na::Vector2<f64>>,
        canvas: &RnoteCanvas,
    ) -> anyhow::Result<()> {
        glib::MainContext::default().spawn_local(clone!(@weak canvas, @weak self as appwindow => async move {
            appwindow.overlays().start_pulsing_progressbar();

            match crate::utils::FileType::lookup_file_type(&file) {
                crate::utils::FileType::RnoteFile => {
                    match file.load_bytes_future().await {
                        Ok((bytes, _)) => {
                            if let Err(e) = canvas.load_in_rnote_bytes(bytes.to_vec(), file.path()).await {
                                log::error!("load_in_rnote_bytes() failed with Err: {e:?}");
                                appwindow.overlays().dispatch_toast_error(&gettext("Opening .rnote file failed."));
                            }
                        }
                        Err(e) => log::error!("failed to load bytes, Err: {e:?}"),
                    }
                }
                crate::utils::FileType::VectorImageFile => {
                    match file.load_bytes_future().await {
                        Ok((bytes, _)) => {
                            if let Err(e) = canvas.load_in_vectorimage_bytes(bytes.to_vec(), target_pos).await {
                                log::error!("load_in_vectorimage_bytes() failed with Err: {e:?}");
                                appwindow.overlays().dispatch_toast_error(&gettext("Opening vector image file failed."));
                            }
                        }
                        Err(e) => log::error!("failed to load bytes, Err: {e:?}"),
                    }
                }
                crate::utils::FileType::BitmapImageFile => {
                    match file.load_bytes_future().await {
                        Ok((bytes, _)) => {
                            if let Err(e) = canvas.load_in_bitmapimage_bytes(bytes.to_vec(), target_pos).await {
                                log::error!("load_in_bitmapimage_bytes() failed with Err: {e:?}");
                                appwindow.overlays().dispatch_toast_error(&gettext("Opening bitmap image file failed."));
                            }
                        }
                        Err(e) => log::error!("failed to load bytes, Err: {e:?}"),
                    }
                }
                crate::utils::FileType::XoppFile => {
                    match file.load_bytes_future().await {
                        Ok((bytes, _)) => {
                            if let Err(e) = canvas.load_in_xopp_bytes(bytes.to_vec()).await {
                                log::error!("load_in_xopp_bytes() failed with Err: {e:?}");
                                appwindow.overlays().dispatch_toast_error(&gettext("Opening Xournal++ file failed."));
                            }
                        }
                        Err(e) => log::error!("failed to load bytes, Err: {e:?}"),
                    }
                }
                crate::utils::FileType::PdfFile => {
                    match file.load_bytes_future().await {
                        Ok((bytes, _)) => {
                            if let Err(e) = canvas.load_in_pdf_bytes(bytes.to_vec(), target_pos, None).await {
                                log::error!("load_in_pdf_bytes() failed with Err: {e:?}");
                                appwindow.overlays().dispatch_toast_error(&gettext("Opening PDF file failed."));
                            }
                        }
                        Err(e) => log::error!("failed to load bytes, Err: {e:?}"),
                    }
                }
                crate::utils::FileType::Folder => {
                    log::error!("tried to open a folder as a file.");
                    appwindow.overlays()
                        .dispatch_toast_error(&gettext("Error: Tried opening folder as file"));
                }
                crate::utils::FileType::Unsupported => {
                    log::error!("tried to open a unsupported file type.");
                    appwindow.overlays()
                        .dispatch_toast_error(&gettext("Failed to open file: Unsupported file type."));
                }
            }

            appwindow.overlays().finish_progressbar();
        }));

        Ok(())
    }

    /// Refreshes the UI from the engine state from the given tab page.
    pub(crate) fn refresh_ui_from_engine(&self, active_tab: &RnoteCanvasWrapper) {
        let canvas = active_tab.canvas();

        // Avoids already borrowed
        let format = canvas.engine().borrow().document.format;
        let doc_layout = canvas.engine().borrow().document.layout;
        let pen_sounds = canvas.engine().borrow().pen_sounds();
        let pen_style = canvas
            .engine()
            .borrow()
            .penholder
            .current_pen_style_w_override();

        // Undo / redo
        let can_undo = canvas.engine().borrow().can_undo();
        let can_redo = canvas.engine().borrow().can_redo();

        self.mainheader().undo_button().set_sensitive(can_undo);
        self.mainheader().redo_button().set_sensitive(can_redo);

        // we change the state through the actions, because they themselves hold state. ( e.g. used to display tickboxes for boolean actions )
        adw::prelude::ActionGroupExt::activate_action(
            self,
            "doc-layout",
            Some(&doc_layout.nick().to_variant()),
        );
        adw::prelude::ActionGroupExt::change_action_state(
            self,
            "pen-sounds",
            &pen_sounds.to_variant(),
        );
        adw::prelude::ActionGroupExt::change_action_state(
            self,
            "format-borders",
            &format.show_borders.to_variant(),
        );
        adw::prelude::ActionGroupExt::change_action_state(
            self,
            "pen-style",
            &pen_style.to_variant(),
        );

        // Current pen
        match pen_style {
            PenStyle::Brush => {
                self.overlays().brush_toggle().set_active(true);
                self.overlays()
                    .penssidebar()
                    .sidebar_stack()
                    .set_visible_child_name("brush_page");

                let style = canvas.engine().borrow().pens_config.brush_config.style;
                match style {
                    BrushStyle::Marker => {
                        let stroke_color = canvas
                            .engine()
                            .borrow()
                            .pens_config
                            .brush_config
                            .marker_options
                            .stroke_color
                            .unwrap_or(Color::TRANSPARENT);
                        let fill_color = canvas
                            .engine()
                            .borrow()
                            .pens_config
                            .brush_config
                            .marker_options
                            .fill_color
                            .unwrap_or(Color::TRANSPARENT);
                        self.overlays()
                            .colorpicker()
                            .set_stroke_color(gdk::RGBA::from_compose_color(stroke_color));
                        self.overlays()
                            .colorpicker()
                            .set_fill_color(gdk::RGBA::from_compose_color(fill_color));
                    }
                    BrushStyle::Solid => {
                        let stroke_color = canvas
                            .engine()
                            .borrow()
                            .pens_config
                            .brush_config
                            .solid_options
                            .stroke_color
                            .unwrap_or(Color::TRANSPARENT);
                        let fill_color = canvas
                            .engine()
                            .borrow()
                            .pens_config
                            .brush_config
                            .solid_options
                            .fill_color
                            .unwrap_or(Color::TRANSPARENT);
                        self.overlays()
                            .colorpicker()
                            .set_stroke_color(gdk::RGBA::from_compose_color(stroke_color));
                        self.overlays()
                            .colorpicker()
                            .set_fill_color(gdk::RGBA::from_compose_color(fill_color));
                    }
                    BrushStyle::Textured => {
                        let stroke_color = canvas
                            .engine()
                            .borrow()
                            .pens_config
                            .brush_config
                            .textured_options
                            .stroke_color
                            .unwrap_or(Color::TRANSPARENT);
                        self.overlays()
                            .colorpicker()
                            .set_stroke_color(gdk::RGBA::from_compose_color(stroke_color));
                    }
                }
            }
            PenStyle::Shaper => {
                self.overlays().shaper_toggle().set_active(true);
                self.overlays()
                    .penssidebar()
                    .sidebar_stack()
                    .set_visible_child_name("shaper_page");

                let style = canvas.engine().borrow().pens_config.shaper_config.style;
                match style {
                    ShaperStyle::Smooth => {
                        let stroke_color = canvas
                            .engine()
                            .borrow()
                            .pens_config
                            .shaper_config
                            .smooth_options
                            .stroke_color
                            .unwrap_or(Color::TRANSPARENT);
                        let fill_color = canvas
                            .engine()
                            .borrow()
                            .pens_config
                            .shaper_config
                            .smooth_options
                            .fill_color
                            .unwrap_or(Color::TRANSPARENT);
                        self.overlays()
                            .colorpicker()
                            .set_stroke_color(gdk::RGBA::from_compose_color(stroke_color));
                        self.overlays()
                            .colorpicker()
                            .set_fill_color(gdk::RGBA::from_compose_color(fill_color));
                    }
                    ShaperStyle::Rough => {
                        let stroke_color = canvas
                            .engine()
                            .borrow()
                            .pens_config
                            .shaper_config
                            .rough_options
                            .stroke_color
                            .unwrap_or(Color::TRANSPARENT);
                        let fill_color = canvas
                            .engine()
                            .borrow()
                            .pens_config
                            .shaper_config
                            .rough_options
                            .fill_color
                            .unwrap_or(Color::TRANSPARENT);
                        self.overlays()
                            .colorpicker()
                            .set_stroke_color(gdk::RGBA::from_compose_color(stroke_color));
                        self.overlays()
                            .colorpicker()
                            .set_fill_color(gdk::RGBA::from_compose_color(fill_color));
                    }
                }
            }
            PenStyle::Typewriter => {
                self.overlays().typewriter_toggle().set_active(true);
                self.overlays()
                    .penssidebar()
                    .sidebar_stack()
                    .set_visible_child_name("typewriter_page");

                let text_color = canvas
                    .engine()
                    .borrow()
                    .pens_config
                    .typewriter_config
                    .text_style
                    .color;
                self.overlays()
                    .colorpicker()
                    .set_stroke_color(gdk::RGBA::from_compose_color(text_color));
            }
            PenStyle::Eraser => {
                self.overlays().eraser_toggle().set_active(true);
                self.overlays()
                    .penssidebar()
                    .sidebar_stack()
                    .set_visible_child_name("eraser_page");
            }
            PenStyle::Selector => {
                self.overlays().selector_toggle().set_active(true);
                self.overlays()
                    .penssidebar()
                    .sidebar_stack()
                    .set_visible_child_name("selector_page");
            }
            PenStyle::Tools => {
                self.overlays().tools_toggle().set_active(true);
                self.overlays()
                    .penssidebar()
                    .sidebar_stack()
                    .set_visible_child_name("tools_page");
            }
        }

        self.overlays()
            .penssidebar()
            .brush_page()
            .refresh_ui(active_tab);
        self.overlays()
            .penssidebar()
            .shaper_page()
            .refresh_ui(active_tab);
        self.overlays()
            .penssidebar()
            .typewriter_page()
            .refresh_ui(active_tab);
        self.overlays()
            .penssidebar()
            .eraser_page()
            .refresh_ui(active_tab);
        self.overlays()
            .penssidebar()
            .selector_page()
            .refresh_ui(active_tab);
        self.overlays()
            .penssidebar()
            .tools_page()
            .refresh_ui(active_tab);
        self.settings_panel().refresh_ui(active_tab);
        self.refresh_titles(active_tab);
    }

    /// Syncs the state from the previous active tab and the current one. Used when the selected tab changes.
    pub(crate) fn sync_state_between_tabs(
        &self,
        prev_tab: &adw::TabPage,
        active_tab: &adw::TabPage,
    ) {
        if prev_tab == active_tab {
            return;
        }
        let prev_canvas_wrapper = prev_tab.child().downcast::<RnoteCanvasWrapper>().unwrap();
        let prev_canvas = prev_canvas_wrapper.canvas();
        let active_canvas_wrapper = active_tab.child().downcast::<RnoteCanvasWrapper>().unwrap();
        let active_canvas = active_canvas_wrapper.canvas();
        let mut widget_flags = WidgetFlags::default();

        // extra scope for engine borrow
        {
            let prev_engine = prev_canvas.engine();
            let prev_engine = prev_engine.borrow();
            let active_engine = active_canvas.engine();
            let mut active_engine = active_engine.borrow_mut();

            active_engine.pens_config = prev_engine.pens_config.clone();
            active_engine.penholder.shortcuts = prev_engine.penholder.shortcuts.clone();
            active_engine.penholder.pen_mode_state = prev_engine.penholder.pen_mode_state.clone();
            widget_flags.merge(active_engine.change_pen_style(prev_engine.penholder.pen_style()));
            // ensures a clean state for the current pen
            widget_flags.merge(active_engine.reinstall_pen_current_style());
            active_engine.import_prefs = prev_engine.import_prefs;
            active_engine.export_prefs = prev_engine.export_prefs;
            active_engine.set_pen_sounds(prev_engine.pen_sounds(), Some(config::DATADIR.into()));
            active_engine.visual_debug = prev_engine.visual_debug;
        }

        self.handle_widget_flags(widget_flags, &active_canvas);
    }
}
