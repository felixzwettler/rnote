use adw::prelude::*;
use gtk4::{
    gdk, glib, glib::clone, subclass::prelude::*, CompositeTemplate, Image, ListBox, MenuButton,
    Popover, SpinButton,
};
use num_traits::cast::ToPrimitive;

use rnote_compose::builders::PenPathBuilderType;
use rnote_compose::style::PressureCurve;
use rnote_engine::pens::Brush;

use crate::{appwindow::RnoteAppWindow, ColorPicker};
use rnote_compose::style::textured::{TexturedDotsDistribution, TexturedOptions};
use rnote_engine::pens::brush::BrushStyle;
use rnote_engine::utils::GdkRGBAHelpers;

mod imp {
    use super::*;

    #[derive(Default, Debug, CompositeTemplate)]
    #[template(resource = "/com/github/flxzt/rnote/ui/penssidebar/brushpage.ui")]
    pub struct BrushPage {
        #[template_child]
        pub width_spinbutton: TemplateChild<SpinButton>,
        #[template_child]
        pub colorpicker: TemplateChild<ColorPicker>,
        #[template_child]
        pub brushstyle_menubutton: TemplateChild<MenuButton>,
        #[template_child]
        pub brushstyle_image: TemplateChild<Image>,
        #[template_child]
        pub brushstyle_listbox: TemplateChild<ListBox>,
        #[template_child]
        pub brushstyle_marker_row: TemplateChild<adw::ActionRow>,
        #[template_child]
        pub brushstyle_solid_row: TemplateChild<adw::ActionRow>,
        #[template_child]
        pub brushstyle_textured_row: TemplateChild<adw::ActionRow>,
        #[template_child]
        pub brushconfig_menubutton: TemplateChild<MenuButton>,
        #[template_child]
        pub brushconfig_popover: TemplateChild<Popover>,
        #[template_child]
        pub brush_buildertype_listbox: TemplateChild<ListBox>,
        #[template_child]
        pub brush_buildertype_simple: TemplateChild<adw::ActionRow>,
        #[template_child]
        pub brush_buildertype_curved: TemplateChild<adw::ActionRow>,
        #[template_child]
        pub brush_buildertype_modeled: TemplateChild<adw::ActionRow>,
        #[template_child]
        pub solidstyle_pressure_curves_row: TemplateChild<adw::ComboRow>,
        #[template_child]
        pub texturedstyle_density_spinbutton: TemplateChild<SpinButton>,
        #[template_child]
        pub texturedstyle_distribution_row: TemplateChild<adw::ComboRow>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for BrushPage {
        const NAME: &'static str = "BrushPage";
        type Type = super::BrushPage;
        type ParentType = gtk4::Widget;

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for BrushPage {
        fn constructed(&self) {
            self.parent_constructed();
        }

        fn dispose(&self) {
            while let Some(child) = self.instance().first_child() {
                child.unparent();
            }
        }
    }

    impl WidgetImpl for BrushPage {}
}

glib::wrapper! {
    pub struct BrushPage(ObjectSubclass<imp::BrushPage>)
        @extends gtk4::Widget;
}

impl Default for BrushPage {
    fn default() -> Self {
        Self::new()
    }
}

impl BrushPage {
    pub fn new() -> Self {
        glib::Object::new(&[])
    }

    pub fn colorpicker(&self) -> ColorPicker {
        self.imp().colorpicker.get()
    }

    pub fn brushstyle_menubutton(&self) -> MenuButton {
        self.imp().brushstyle_menubutton.get()
    }

    pub fn brushconfig_menubutton(&self) -> MenuButton {
        self.imp().brushconfig_menubutton.get()
    }

    pub fn solidstyle_pressure_curve(&self) -> PressureCurve {
        PressureCurve::try_from(self.imp().solidstyle_pressure_curves_row.get().selected()).unwrap()
    }

    pub fn set_solidstyle_pressure_curve(&self, pressure_curve: PressureCurve) {
        let position = pressure_curve.to_u32().unwrap();

        self.imp()
            .solidstyle_pressure_curves_row
            .get()
            .set_selected(position);
    }

    pub fn texturedstyle_dots_distribution(&self) -> TexturedDotsDistribution {
        TexturedDotsDistribution::try_from(
            self.imp().texturedstyle_distribution_row.get().selected(),
        )
        .unwrap()
    }

    pub fn set_texturedstyle_distribution_variant(&self, distribution: TexturedDotsDistribution) {
        let position = distribution.to_u32().unwrap();

        self.imp()
            .texturedstyle_distribution_row
            .get()
            .set_selected(position);
    }

    pub fn init(&self, appwindow: &RnoteAppWindow) {
        let imp = self.imp();

        imp.width_spinbutton.set_increments(0.1, 2.0);
        imp.width_spinbutton
            .set_range(Brush::STROKE_WIDTH_MIN, Brush::STROKE_WIDTH_MAX);
        // Must be after set_range() !
        imp.width_spinbutton.set_value(Brush::STROKE_WIDTH_DEFAULT);

        imp.colorpicker.connect_notify_local(
            Some("current-color"),
            clone!(@weak appwindow => move |colorpicker, _paramspec| {
                let color = colorpicker.property::<gdk::RGBA>("current-color").into_compose_color();
                let brush_style = appwindow.canvas().engine().borrow_mut().penholder.brush.style;

                match brush_style {
                    BrushStyle::Marker => appwindow.canvas().engine().borrow_mut().penholder.brush.marker_options.stroke_color = Some(color),
                    BrushStyle::Solid => appwindow.canvas().engine().borrow_mut().penholder.brush.solid_options.stroke_color = Some(color),
                    BrushStyle::Textured => appwindow.canvas().engine().borrow_mut().penholder.brush.textured_options.stroke_color = Some(color),
                }
            }),
        );

        imp.width_spinbutton.connect_value_changed(
            clone!(@weak appwindow => move |brush_widthscale_spinbutton| {
                let brush_style = appwindow.canvas().engine().borrow_mut().penholder.brush.style;

                match brush_style {
                    BrushStyle::Marker => appwindow.canvas().engine().borrow_mut().penholder.brush.marker_options.stroke_width = brush_widthscale_spinbutton.value(),
                    BrushStyle::Solid => appwindow.canvas().engine().borrow_mut().penholder.brush.solid_options.stroke_width = brush_widthscale_spinbutton.value(),
                    BrushStyle::Textured => appwindow.canvas().engine().borrow_mut().penholder.brush.textured_options.stroke_width = brush_widthscale_spinbutton.value(),
                }
            }),
        );

        imp.brushstyle_listbox.connect_row_selected(
            clone!(@weak self as brushpage, @weak appwindow => move |_brushstyle_listbox, selected_row| {
                if let Some(selected_row) = selected_row.map(|selected_row| {selected_row.downcast_ref::<adw::ActionRow>().unwrap()}) {
                    {
                        let engine = appwindow.canvas().engine();
                        let engine = &mut *engine.borrow_mut();

                        engine.penholder.brush.style = BrushStyle::try_from(selected_row.index() as u32).unwrap_or_default();

                        // Overwrite the color, but not the width
                        match engine.penholder.brush.style {
                            BrushStyle::Marker => {
                                engine.penholder.brush.marker_options.stroke_color = Some(brushpage.colorpicker().current_color());
                            },
                            BrushStyle::Solid => {
                                engine.penholder.brush.solid_options.stroke_color = Some(brushpage.colorpicker().current_color());
                            },
                            BrushStyle::Textured => {
                                engine.penholder.brush.textured_options.stroke_color = Some(brushpage.colorpicker().current_color());
                            },
                        }
                    }

                    // Need to refresh the whole page, because changing the style affects multiple widgets
                    brushpage.refresh_ui(&appwindow);
                }
            }),
        );

        // Builder type
        imp.brush_buildertype_listbox.connect_row_selected(
            clone!(@weak self as brushpage, @weak appwindow => move |_, selected_row| {
                if let Some(selected_row) = selected_row.map(|selected_row| {selected_row.downcast_ref::<adw::ActionRow>().unwrap()}) {
                    appwindow.canvas().engine().borrow_mut().penholder.brush.builder_type = PenPathBuilderType::try_from(selected_row.index() as u32).unwrap_or_default();
                }
            }),
        );

        // Solid style
        // Pressure curve
        imp.solidstyle_pressure_curves_row.get().connect_selected_notify(clone!(@weak self as brushpage, @weak appwindow => move |_smoothstyle_pressure_curves_row| {
            appwindow.canvas().engine().borrow_mut().penholder.brush.solid_options.pressure_curve = brushpage.solidstyle_pressure_curve();
        }));

        // Textured style
        // Density
        imp.texturedstyle_density_spinbutton
            .get()
            .set_increments(0.1, 2.0);
        imp.texturedstyle_density_spinbutton
            .get()
            .set_range(0.0, f64::MAX);
        imp.texturedstyle_density_spinbutton
            .get()
            .set_value(TexturedOptions::DENSITY_DEFAULT);

        imp.texturedstyle_density_spinbutton.get().connect_value_changed(
            clone!(@weak appwindow => move |texturedstyle_density_adj| {
                appwindow.canvas().engine().borrow_mut().penholder.brush.textured_options.density = texturedstyle_density_adj.value();
            }),
        );

        // dots distribution
        imp.texturedstyle_distribution_row.get().connect_selected_notify(clone!(@weak self as brushpage, @weak appwindow => move |_texturedstyle_distribution_row| {
            appwindow.canvas().engine().borrow_mut().penholder.brush.textured_options.distribution = brushpage.texturedstyle_dots_distribution();
        }));
    }

    pub fn refresh_ui(&self, appwindow: &RnoteAppWindow) {
        let imp = self.imp();
        let brush = appwindow.canvas().engine().borrow().penholder.brush.clone();

        self.set_solidstyle_pressure_curve(brush.solid_options.pressure_curve);
        imp.texturedstyle_density_spinbutton
            .set_value(brush.textured_options.density);
        self.set_texturedstyle_distribution_variant(brush.textured_options.distribution);

        match brush.builder_type {
            PenPathBuilderType::Simple => {
                imp.brush_buildertype_listbox
                    .select_row(Some(&*imp.brush_buildertype_simple));
            }
            PenPathBuilderType::Curved => {
                imp.brush_buildertype_listbox
                    .select_row(Some(&*imp.brush_buildertype_curved));
            }
            PenPathBuilderType::Modeled => {
                imp.brush_buildertype_listbox
                    .select_row(Some(&*imp.brush_buildertype_modeled));
            }
        }

        match brush.style {
            BrushStyle::Marker => {
                imp.brushstyle_listbox
                    .select_row(Some(&*imp.brushstyle_marker_row));
                imp.width_spinbutton
                    .set_value(brush.marker_options.stroke_width);
                imp.colorpicker
                    .set_current_color(brush.marker_options.stroke_color);
                imp.brushstyle_image
                    .set_icon_name(Some("pen-brush-style-marker-symbolic"));
            }
            BrushStyle::Solid => {
                imp.brushstyle_listbox
                    .select_row(Some(&*imp.brushstyle_solid_row));
                imp.width_spinbutton
                    .set_value(brush.solid_options.stroke_width);
                imp.colorpicker
                    .set_current_color(brush.solid_options.stroke_color);
                imp.brushstyle_image
                    .set_icon_name(Some("pen-brush-style-solid-symbolic"));
            }
            BrushStyle::Textured => {
                imp.brushstyle_listbox
                    .select_row(Some(&*imp.brushstyle_textured_row));
                imp.width_spinbutton
                    .set_value(brush.textured_options.stroke_width);
                imp.colorpicker
                    .set_current_color(brush.textured_options.stroke_color);
                imp.brushstyle_image
                    .set_icon_name(Some("pen-brush-style-textured-symbolic"));
            }
        }
    }
}
