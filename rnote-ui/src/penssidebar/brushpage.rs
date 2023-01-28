use adw::prelude::*;
use gtk4::{
    glib, glib::clone, subclass::prelude::*, CompositeTemplate, ListBox, MenuButton, Popover,
    SpinButton,
};
use num_traits::cast::ToPrimitive;
use rnote_compose::builders::PenPathBuilderType;
use rnote_compose::style::textured::{TexturedDotsDistribution, TexturedOptions};
use rnote_compose::style::PressureCurve;
use rnote_engine::pens::pensconfig::brushconfig::{BrushStyle, SolidOptions};
use rnote_engine::pens::pensconfig::BrushConfig;

use crate::{RnoteAppWindow, RnoteCanvasWrapper, StrokeWidthPicker};

mod imp {
    use super::*;

    #[derive(Default, Debug, CompositeTemplate)]
    #[template(resource = "/com/github/flxzt/rnote/ui/penssidebar/brushpage.ui")]
    pub(crate) struct BrushPage {
        #[template_child]
        pub(crate) brushstyle_menubutton: TemplateChild<MenuButton>,
        #[template_child]
        pub(crate) brushstyle_listbox: TemplateChild<ListBox>,
        #[template_child]
        pub(crate) brushstyle_marker_row: TemplateChild<adw::ActionRow>,
        #[template_child]
        pub(crate) brushstyle_solid_row: TemplateChild<adw::ActionRow>,
        #[template_child]
        pub(crate) brushstyle_textured_row: TemplateChild<adw::ActionRow>,
        #[template_child]
        pub(crate) brushconfig_menubutton: TemplateChild<MenuButton>,
        #[template_child]
        pub(crate) brushconfig_popover: TemplateChild<Popover>,
        #[template_child]
        pub(crate) brush_buildertype_listbox: TemplateChild<ListBox>,
        #[template_child]
        pub(crate) brush_buildertype_simple: TemplateChild<adw::ActionRow>,
        #[template_child]
        pub(crate) brush_buildertype_curved: TemplateChild<adw::ActionRow>,
        #[template_child]
        pub(crate) brush_buildertype_modeled: TemplateChild<adw::ActionRow>,
        #[template_child]
        pub(crate) solidstyle_pressure_curves_row: TemplateChild<adw::ComboRow>,
        #[template_child]
        pub(crate) texturedstyle_density_spinbutton: TemplateChild<SpinButton>,
        #[template_child]
        pub(crate) texturedstyle_distribution_row: TemplateChild<adw::ComboRow>,
        #[template_child]
        pub(crate) stroke_width_picker: TemplateChild<StrokeWidthPicker>,
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
    pub(crate) struct BrushPage(ObjectSubclass<imp::BrushPage>)
        @extends gtk4::Widget;
}

impl Default for BrushPage {
    fn default() -> Self {
        Self::new()
    }
}

impl BrushPage {
    pub(crate) fn new() -> Self {
        glib::Object::new(&[])
    }

    pub(crate) fn brushstyle_menubutton(&self) -> MenuButton {
        self.imp().brushstyle_menubutton.get()
    }

    pub(crate) fn brushconfig_menubutton(&self) -> MenuButton {
        self.imp().brushconfig_menubutton.get()
    }

    pub(crate) fn brush_style(&self) -> Option<BrushStyle> {
        BrushStyle::try_from(self.imp().brushstyle_listbox.selected_row()?.index() as u32).ok()
    }

    pub(crate) fn set_brush_style(&self, brush_style: BrushStyle) {
        match brush_style {
            BrushStyle::Marker => self
                .imp()
                .brushstyle_listbox
                .select_row(Some(&*self.imp().brushstyle_marker_row)),
            BrushStyle::Solid => self
                .imp()
                .brushstyle_listbox
                .select_row(Some(&*self.imp().brushstyle_solid_row)),
            BrushStyle::Textured => self
                .imp()
                .brushstyle_listbox
                .select_row(Some(&*self.imp().brushstyle_textured_row)),
        }
    }

    pub(crate) fn buildertype(&self) -> Option<PenPathBuilderType> {
        PenPathBuilderType::try_from(
            self.imp().brush_buildertype_listbox.selected_row()?.index() as u32
        )
        .ok()
    }

    pub(crate) fn set_buildertype(&self, buildertype: PenPathBuilderType) {
        match buildertype {
            PenPathBuilderType::Simple => self
                .imp()
                .brush_buildertype_listbox
                .select_row(Some(&*self.imp().brush_buildertype_simple)),
            PenPathBuilderType::Curved => self
                .imp()
                .brush_buildertype_listbox
                .select_row(Some(&*self.imp().brush_buildertype_curved)),
            PenPathBuilderType::Modeled => self
                .imp()
                .brush_buildertype_listbox
                .select_row(Some(&*self.imp().brush_buildertype_modeled)),
        }
    }

    pub(crate) fn solidstyle_pressure_curve(&self) -> PressureCurve {
        PressureCurve::try_from(self.imp().solidstyle_pressure_curves_row.get().selected()).unwrap()
    }

    pub(crate) fn set_solidstyle_pressure_curve(&self, pressure_curve: PressureCurve) {
        let position = pressure_curve.to_u32().unwrap();

        self.imp()
            .solidstyle_pressure_curves_row
            .get()
            .set_selected(position);
    }

    pub(crate) fn texturedstyle_dots_distribution(&self) -> TexturedDotsDistribution {
        TexturedDotsDistribution::try_from(
            self.imp().texturedstyle_distribution_row.get().selected(),
        )
        .unwrap()
    }

    pub(crate) fn set_texturedstyle_distribution_variant(
        &self,
        distribution: TexturedDotsDistribution,
    ) {
        let position = distribution.to_u32().unwrap();

        self.imp()
            .texturedstyle_distribution_row
            .get()
            .set_selected(position);
    }

    pub(crate) fn stroke_width_picker(&self) -> StrokeWidthPicker {
        self.imp().stroke_width_picker.get()
    }

    pub(crate) fn init(&self, appwindow: &RnoteAppWindow) {
        let imp = self.imp();

        // Stroke width
        imp.stroke_width_picker
            .spinbutton()
            .set_range(BrushConfig::STROKE_WIDTH_MIN, BrushConfig::STROKE_WIDTH_MAX);
        // set value after the range!
        imp.stroke_width_picker
            .set_stroke_width(SolidOptions::default().stroke_width);

        imp.stroke_width_picker.connect_notify_local(
            Some("stroke-width"),
            clone!(@weak self as brushpage, @weak appwindow => move |picker, _| {
                let stroke_width = picker.stroke_width();
                let engine = appwindow.active_tab().canvas().engine();
                let engine = &mut *engine.borrow_mut();

                match engine.pens_config.brush_config.style {
                    BrushStyle::Marker => {
                        engine.pens_config.brush_config.marker_options.stroke_width = stroke_width;
                    },
                    BrushStyle::Solid => {
                        engine.pens_config.brush_config.solid_options.stroke_width = stroke_width;
                    },
                    BrushStyle::Textured => {
                        engine.pens_config.brush_config.textured_options.stroke_width = stroke_width;
                    },
                }
            }),
        );

        // Style
        imp.brushstyle_listbox.connect_row_selected(
            clone!(@weak self as brushpage, @weak appwindow => move |_, _| {
                if let Some(brush_style) = brushpage.brush_style() {
                    appwindow.active_tab().canvas().engine().borrow_mut().pens_config.brush_config.style = brush_style;
                    brushpage.stroke_width_picker().deselect_setters();

                    match brush_style {
                        BrushStyle::Marker => {
                            let stroke_width = appwindow.active_tab().canvas().engine().borrow_mut().pens_config.brush_config.marker_options.stroke_width;
                            brushpage.imp().stroke_width_picker.set_stroke_width(stroke_width);
                            brushpage.imp().brushstyle_menubutton.set_icon_name("pen-brush-style-marker-symbolic");
                        },
                        BrushStyle::Solid => {
                            let stroke_width = appwindow.active_tab().canvas().engine().borrow_mut().pens_config.brush_config.solid_options.stroke_width;
                            brushpage.imp().stroke_width_picker.set_stroke_width(stroke_width);
                            brushpage.imp().brushstyle_menubutton.set_icon_name("pen-brush-style-solid-symbolic");
                        },
                        BrushStyle::Textured => {
                            let stroke_width = appwindow.active_tab().canvas().engine().borrow_mut().pens_config.brush_config.textured_options.stroke_width;
                            brushpage.imp().stroke_width_picker.set_stroke_width(stroke_width);
                            brushpage.imp().brushstyle_menubutton.set_icon_name("pen-brush-style-textured-symbolic");
                        },
                    }
                }
            }),
        );

        // Builder type
        imp.brush_buildertype_listbox.connect_row_selected(
            clone!(@weak self as brushpage, @weak appwindow => move |_, _| {
                if let Some(buildertype) = brushpage.buildertype() {
                    appwindow.active_tab().canvas().engine().borrow_mut().pens_config.brush_config.builder_type = buildertype;
                }
            }),
        );

        // Solid style
        // Pressure curve
        imp.solidstyle_pressure_curves_row.get().connect_selected_notify(clone!(@weak self as brushpage, @weak appwindow => move |_smoothstyle_pressure_curves_row| {
            appwindow.active_tab().canvas().engine().borrow_mut().pens_config.brush_config.solid_options.pressure_curve = brushpage.solidstyle_pressure_curve();
        }));

        // Textured style
        // Density
        imp.texturedstyle_density_spinbutton
            .get()
            .set_increments(0.1, 2.0);
        imp.texturedstyle_density_spinbutton
            .get()
            .set_range(TexturedOptions::DENSITY_MIN, TexturedOptions::DENSITY_MAX);
        // set value after the range!
        imp.texturedstyle_density_spinbutton
            .get()
            .set_value(TexturedOptions::default().density);

        imp.texturedstyle_density_spinbutton.get().connect_value_changed(
            clone!(@weak appwindow => move |spinbutton| {
                appwindow.active_tab().canvas().engine().borrow_mut().pens_config.brush_config.textured_options.density = spinbutton.value();
            }),
        );

        // dots distribution
        imp.texturedstyle_distribution_row.get().connect_selected_notify(clone!(@weak self as brushpage, @weak appwindow => move |_texturedstyle_distribution_row| {
            appwindow.active_tab().canvas().engine().borrow_mut().pens_config.brush_config.textured_options.distribution = brushpage.texturedstyle_dots_distribution();
        }));
    }

    pub(crate) fn refresh_ui(&self, active_tab: &RnoteCanvasWrapper) {
        let imp = self.imp();
        let brush_config = active_tab
            .canvas()
            .engine()
            .borrow()
            .pens_config
            .brush_config
            .clone();

        self.set_solidstyle_pressure_curve(brush_config.solid_options.pressure_curve);
        imp.texturedstyle_density_spinbutton
            .set_value(brush_config.textured_options.density);
        self.set_texturedstyle_distribution_variant(brush_config.textured_options.distribution);

        self.set_brush_style(brush_config.style);
        self.set_buildertype(brush_config.builder_type);

        match brush_config.style {
            BrushStyle::Marker => {
                imp.stroke_width_picker
                    .set_stroke_width(brush_config.marker_options.stroke_width);
            }
            BrushStyle::Solid => {
                imp.stroke_width_picker
                    .set_stroke_width(brush_config.solid_options.stroke_width);
            }
            BrushStyle::Textured => {
                imp.stroke_width_picker
                    .set_stroke_width(brush_config.textured_options.stroke_width);
            }
        }
    }
}
