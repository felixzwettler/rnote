use std::path::PathBuf;

use crate::appwindow::RnoteAppWindow;
use crate::config;
use gtk4::{gdk, glib};

use adw::prelude::*;

impl RnoteAppWindow {
    /// Settings binds
    pub(crate) fn setup_settings_binds(&self) {
        let app = self.app();

        // Color scheme
        self.app_settings()
            .bind("color-scheme", &app.style_manager(), "color-scheme")
            .mapping(|variant, _| {
                let value = variant.get::<String>().unwrap();

                match value.as_str() {
                    "default" => Some(adw::ColorScheme::Default.to_value()),
                    "force-light" => Some(adw::ColorScheme::ForceLight.to_value()),
                    "force-dark" => Some(adw::ColorScheme::ForceDark.to_value()),
                    _ => {
                        log::error!(
                            "mapping color-scheme to setting failed, invalid str {}",
                            value.as_str()
                        );
                        None
                    }
                }
            })
            .set_mapping(|value, _| match value.get::<adw::ColorScheme>().unwrap() {
                adw::ColorScheme::Default => Some(String::from("default").to_variant()),
                adw::ColorScheme::ForceLight => Some(String::from("force-light").to_variant()),
                adw::ColorScheme::ForceDark => Some(String::from("force-dark").to_variant()),
                _ => None,
            })
            .build();

        // autosave
        self.app_settings()
            .bind("autosave", self, "autosave")
            .build();

        // autosave interval secs
        self.app_settings()
            .bind("autosave-interval-secs", self, "autosave-interval-secs")
            .build();

        // righthanded
        self.app_settings()
            .bind("righthanded", self, "righthanded")
            .build();

        // touch drawing
        self.app_settings()
            .bind("touch-drawing", self, "touch-drawing")
            .build();

        // permanently hide canvas scrollbars
        self.app_settings()
            .bind(
                "show-scrollbars",
                &self.settings_panel().general_show_scrollbars_switch(),
                "active",
            )
            .build();

        // regular cursor
        self.app_settings()
            .bind(
                "regular-cursor",
                &self.settings_panel().general_regular_cursor_picker(),
                "picked",
            )
            .build();

        // drawing cursor
        self.app_settings()
            .bind(
                "drawing-cursor",
                &self.settings_panel().general_drawing_cursor_picker(),
                "picked",
            )
            .build();

        // colorpicker palette
        let colorsetter_mapping = |var: &glib::Variant, _: glib::Type| {
            let color = var.get::<(f64, f64, f64, f64)>()?;
            Some(
                gdk::RGBA::new(
                    color.0 as f32,
                    color.1 as f32,
                    color.2 as f32,
                    color.3 as f32,
                )
                .to_value(),
            )
        };
        let colorsetter_set_mapping = |val: &glib::Value, _: glib::VariantType| {
            let color = val.get::<gdk::RGBA>().ok()?;
            Some(
                (
                    color.red() as f64,
                    color.green() as f64,
                    color.blue() as f64,
                    color.alpha() as f64,
                )
                    .to_variant(),
            )
        };

        self.app_settings()
            .bind(
                "colorpicker-color-1",
                &self.overlays().colorpicker().setter_1(),
                "color",
            )
            .mapping(colorsetter_mapping)
            .set_mapping(colorsetter_set_mapping)
            .build();
        self.app_settings()
            .bind(
                "colorpicker-color-2",
                &self.overlays().colorpicker().setter_2(),
                "color",
            )
            .mapping(colorsetter_mapping)
            .set_mapping(colorsetter_set_mapping)
            .build();
        self.app_settings()
            .bind(
                "colorpicker-color-3",
                &self.overlays().colorpicker().setter_3(),
                "color",
            )
            .mapping(colorsetter_mapping)
            .set_mapping(colorsetter_set_mapping)
            .build();
        self.app_settings()
            .bind(
                "colorpicker-color-4",
                &self.overlays().colorpicker().setter_4(),
                "color",
            )
            .mapping(colorsetter_mapping)
            .set_mapping(colorsetter_set_mapping)
            .build();
        self.app_settings()
            .bind(
                "colorpicker-color-5",
                &self.overlays().colorpicker().setter_5(),
                "color",
            )
            .mapping(colorsetter_mapping)
            .set_mapping(colorsetter_set_mapping)
            .build();
        self.app_settings()
            .bind(
                "colorpicker-color-6",
                &self.overlays().colorpicker().setter_6(),
                "color",
            )
            .mapping(colorsetter_mapping)
            .set_mapping(colorsetter_set_mapping)
            .build();
        self.app_settings()
            .bind(
                "colorpicker-color-7",
                &self.overlays().colorpicker().setter_7(),
                "color",
            )
            .mapping(colorsetter_mapping)
            .set_mapping(colorsetter_set_mapping)
            .build();
        self.app_settings()
            .bind(
                "colorpicker-color-8",
                &self.overlays().colorpicker().setter_8(),
                "color",
            )
            .mapping(colorsetter_mapping)
            .set_mapping(colorsetter_set_mapping)
            .build();

        // brush stroke widths
        self.app_settings()
            .bind(
                "brush-width-1",
                &self
                    .overlays()
                    .penssidebar()
                    .brush_page()
                    .stroke_width_picker()
                    .setter_1(),
                "stroke-width",
            )
            .build();
        self.app_settings()
            .bind(
                "brush-width-2",
                &self
                    .overlays()
                    .penssidebar()
                    .brush_page()
                    .stroke_width_picker()
                    .setter_2(),
                "stroke-width",
            )
            .build();
        self.app_settings()
            .bind(
                "brush-width-3",
                &self
                    .overlays()
                    .penssidebar()
                    .brush_page()
                    .stroke_width_picker()
                    .setter_3(),
                "stroke-width",
            )
            .build();

        // shaper stroke widths
        self.app_settings()
            .bind(
                "shaper-width-1",
                &self
                    .overlays()
                    .penssidebar()
                    .shaper_page()
                    .stroke_width_picker()
                    .setter_1(),
                "stroke-width",
            )
            .build();
        self.app_settings()
            .bind(
                "shaper-width-2",
                &self
                    .overlays()
                    .penssidebar()
                    .shaper_page()
                    .stroke_width_picker()
                    .setter_2(),
                "stroke-width",
            )
            .build();
        self.app_settings()
            .bind(
                "shaper-width-3",
                &self
                    .overlays()
                    .penssidebar()
                    .shaper_page()
                    .stroke_width_picker()
                    .setter_3(),
                "stroke-width",
            )
            .build();
    }

    /// load settings at start that are not bound in setup_settings. Setting changes through gsettings / dconf might not be applied until app restarts
    pub(crate) fn load_settings(&self) {
        let _app = self.app();

        // appwindow
        {
            let window_width = self.app_settings().int("window-width");
            let window_height = self.app_settings().int("window-height");
            let is_maximized = self.app_settings().boolean("is-maximized");

            self.set_default_size(window_width, window_height);

            if is_maximized {
                self.maximize();
            }

            self.flap_box()
                .set_width_request(self.app_settings().int("flap-width"));
        }

        // color scheme
        // Set the action menu, as the style manager colorscheme property may not be changed from the binding at startup when opening a second window (FIXME: why?)
        let color_scheme = self.app_settings().string("color-scheme");
        self.app()
            .activate_action("color-scheme", Some(&color_scheme.to_variant()));

        {
            // Workspaces bar
            self.workspacebrowser()
                .workspacesbar()
                .load_from_settings(&self.app_settings());
        }

        {
            let canvas = self.active_tab().canvas();
            // load engine config
            let engine_config = self.app_settings().string("engine-config");
            let widget_flags = match canvas
                .engine()
                .borrow_mut()
                .load_engine_config(&engine_config, Some(PathBuf::from(config::PKGDATADIR)))
            {
                Err(e) => {
                    // On first app startup the engine config is empty, so we don't log an error
                    if engine_config.is_empty() {
                        log::debug!("did not load `engine-config` from settings, was empty");
                    } else {
                        log::error!("failed to load `engine-config` from settings, Err: {e:?}");
                    }
                    None
                }
                Ok(widget_flags) => Some(widget_flags),
            };

            // Avoiding already borrowed
            if let Some(widget_flags) = widget_flags {
                self.handle_widget_flags(widget_flags, &canvas);
            }
        }
    }

    /// Save all settings at shutdown that are not bound in setup_settings
    pub(crate) fn save_to_settings(&self) -> anyhow::Result<()> {
        {
            // Appwindow
            self.app_settings().set_int("window-width", self.width())?;
            self.app_settings()
                .set_int("window-height", self.height())?;
            self.app_settings()
                .set_boolean("is-maximized", self.is_maximized())?;
            self.app_settings()
                .set_int("flap-width", self.flap_box().width())?;
        }

        {
            // Save engine config
            self.save_engine_config_active_tab()?;
        }

        Ok(())
    }
}
