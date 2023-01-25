mod appsettings;
mod appwindowactions;
mod imp;

use adw::{prelude::*, subclass::prelude::*};
use gettextrs::gettext;
use gtk4::{gio, glib, glib::clone, Application, Box, Button, FileChooserNative, IconTheme};
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

        // A first canvas. Must! come before binding the settings
        self.add_initial_tab();

        // add icon theme resource path because automatic lookup does not work in the devel build.
        let app_icon_theme = IconTheme::for_display(&self.display());
        app_icon_theme.add_resource_path((String::from(config::APP_IDPATH) + "icons").as_str());

        // actions and settings AFTER widget inits
        self.setup_actions();
        self.setup_action_accels();
        self.setup_settings_binds();

        // Load settings
        self.load_settings();

        // Periodically save engine config
        if let Some(removed_id) = self.imp().periodic_configsave_source_id.borrow_mut().replace(
            glib::source::timeout_add_seconds_local(
                Self::PERIODIC_CONFIGSAVE_INTERVAL, clone!(@weak self as appwindow => @default-return glib::source::Continue(false), move || {
                    if let Err(e) = appwindow.save_engine_config_active_tab() {
                        log::error!("saving engine config in periodic task failed with Err: {e:?}");
                    }

                    glib::source::Continue(true)
        }))) {
            removed_id.remove();
        }

        self.init_misc();
    }

    // Anything that needs to be done right before showing the appwindow
    pub(crate) fn init_misc(&self) {
        // Set undo / redo as not sensitive as default ( setting it in .ui file did not work for some reason )
        self.mainheader().undo_button().set_sensitive(false);
        self.mainheader().redo_button().set_sensitive(false);

        // rerender the canvas
        self.active_tab().canvas().regenerate_background_pattern();
        self.active_tab().canvas().update_engine_rendering();

        adw::prelude::ActionGroupExt::activate_action(self, "refresh-ui-from-engine", None);
    }

    /// Called to close the window
    pub(crate) fn close_force(&self) {
        // Saving all state
        if let Err(e) = self.save_to_settings() {
            log::error!("Failed to save appwindow to settings, with Err: {e:?}");
        }

        // Closing the state tasks channel receiver
        if let Err(e) = self
            .active_tab()
            .canvas()
            .engine()
            .borrow()
            .tasks_tx()
            .unbounded_send(EngineTask::Quit)
        {
            log::error!("failed to send StateTask::Quit on store tasks_tx, Err: {e:?}");
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
            adw::prelude::ActionGroupExt::activate_action(self, "refresh-ui-from-engine", None);
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

    pub(crate) fn save_engine_config_active_tab(&self) -> anyhow::Result<()> {
        let engine_config = self
            .active_tab()
            .canvas()
            .engine()
            .borrow()
            .export_engine_config_as_json()?;
        self.app_settings()
            .set_string("engine-config", engine_config.as_str())?;

        Ok(())
    }

    /// Get the active (selected) tab page. If there is none (which should only be the case on appwindow startup), we create one
    pub(crate) fn active_tab_page(&self) -> adw::TabPage {
        // We always create a single page, if there is none initially
        self.imp()
            .overlays
            .tabview()
            .selected_page()
            .unwrap_or_else(|| self.new_tab())
    }

    /// Get the active (selected) tab page child. If there is none (which should only be the case on appwindow startup), we create one
    pub(crate) fn active_tab(&self) -> RnoteCanvasWrapper {
        self.active_tab_page()
            .child()
            .downcast::<RnoteCanvasWrapper>()
            .unwrap()
    }

    /// adds the initial tab to the tabview
    fn add_initial_tab(&self) -> adw::TabPage {
        let new_wrapper = RnoteCanvasWrapper::new();
        let page = self.overlays().tabview().append(&new_wrapper);
        self.overlays().tabview().set_selected_page(&page);
        page
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

    /// Requests to close the active tab, or if only one tab is left, the active appwindow
    pub(crate) fn close_active_tab(&self) {
        let active_tab_page = self.active_tab_page();
        if self.overlays().tabview().n_pages() <= 1 {
            // If there is only one tab left, request to close the entire window.
            self.close();
        } else {
            self.overlays().tabview().close_page(&active_tab_page);
        }
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

    pub(crate) fn refresh_titles_active_tab(&self) {
        let canvas = self.active_tab().canvas();

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
}
