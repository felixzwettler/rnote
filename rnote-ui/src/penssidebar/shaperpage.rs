use crate::{appwindow::RnoteAppWindow, colorpicker::ColorPicker};
use adw::{prelude::*, subclass::prelude::*};
use gtk4::{
    gdk, glib, glib::clone, CompositeTemplate, Image, ListBox, MenuButton, Popover, SpinButton,
    Switch,
};
use num_traits::cast::ToPrimitive;

use rnote_compose::builders::{ConstraintRatio, ShapeBuilderType};
use rnote_compose::style::rough::roughoptions::FillStyle;
use rnote_compose::Color;
use rnote_engine::pens::shaper::ShaperStyle;
use rnote_engine::pens::Shaper;
use rnote_engine::utils::GdkRGBAHelpers;

mod imp {

    use super::*;
    #[derive(Default, Debug, CompositeTemplate)]
    #[template(resource = "/com/github/flxzt/rnote/ui/penssidebar/shaperpage.ui")]
    pub(crate) struct ShaperPage {
        #[template_child]
        pub(crate) shaperstyle_menubutton: TemplateChild<MenuButton>,
        #[template_child]
        pub(crate) shaperstyle_image: TemplateChild<Image>,
        #[template_child]
        pub(crate) shaperstyle_listbox: TemplateChild<ListBox>,
        #[template_child]
        pub(crate) shaperstyle_smooth_row: TemplateChild<adw::ActionRow>,
        #[template_child]
        pub(crate) shaperstyle_rough_row: TemplateChild<adw::ActionRow>,
        #[template_child]
        pub(crate) shapeconfig_menubutton: TemplateChild<MenuButton>,
        #[template_child]
        pub(crate) shapeconfig_popover: TemplateChild<Popover>,
        #[template_child]
        pub(crate) roughstyle_fillstyle_row: TemplateChild<adw::ComboRow>,
        #[template_child]
        pub(crate) roughstyle_hachure_angle_spinbutton: TemplateChild<SpinButton>,
        #[template_child]
        pub(crate) width_spinbutton: TemplateChild<SpinButton>,
        #[template_child]
        pub(crate) stroke_colorpicker: TemplateChild<ColorPicker>,
        #[template_child]
        pub(crate) fill_colorpicker: TemplateChild<ColorPicker>,
        #[template_child]
        pub(crate) shapebuildertype_menubutton: TemplateChild<MenuButton>,
        #[template_child]
        pub(crate) shapebuildertype_image: TemplateChild<Image>,
        #[template_child]
        pub(crate) shapebuildertype_listbox: TemplateChild<ListBox>,
        #[template_child]
        pub(crate) shapebuildertype_line_row: TemplateChild<adw::ActionRow>,
        #[template_child]
        pub(crate) shapebuildertype_rectangle_row: TemplateChild<adw::ActionRow>,
        #[template_child]
        pub(crate) shapebuildertype_coordsystem2d_row: TemplateChild<adw::ActionRow>,
        #[template_child]
        pub(crate) shapebuildertype_coordsystem3d_row: TemplateChild<adw::ActionRow>,
        #[template_child]
        pub(crate) shapebuildertype_quadrantcoordsystem2d_row: TemplateChild<adw::ActionRow>,
        #[template_child]
        pub(crate) shapebuildertype_ellipse_row: TemplateChild<adw::ActionRow>,
        #[template_child]
        pub(crate) shapebuildertype_fociellipse_row: TemplateChild<adw::ActionRow>,
        #[template_child]
        pub(crate) shapebuildertype_quadbez_row: TemplateChild<adw::ActionRow>,
        #[template_child]
        pub(crate) shapebuildertype_cubbez_row: TemplateChild<adw::ActionRow>,
        #[template_child]
        pub(crate) constraint_menubutton: TemplateChild<MenuButton>,
        #[template_child]
        pub(crate) constraint_enabled_switch: TemplateChild<Switch>,
        #[template_child]
        pub(crate) constraint_one_to_one_switch: TemplateChild<Switch>,
        #[template_child]
        pub(crate) constraint_three_to_two_switch: TemplateChild<Switch>,
        #[template_child]
        pub(crate) constraint_golden_switch: TemplateChild<Switch>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ShaperPage {
        const NAME: &'static str = "ShaperPage";
        type Type = super::ShaperPage;
        type ParentType = gtk4::Widget;

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for ShaperPage {
        fn constructed(&self) {
            self.parent_constructed();
        }

        fn dispose(&self) {
            while let Some(child) = self.instance().first_child() {
                child.unparent();
            }
        }
    }

    impl WidgetImpl for ShaperPage {}
}

glib::wrapper! {
    pub(crate) struct ShaperPage(ObjectSubclass<imp::ShaperPage>)
        @extends gtk4::Widget;
}

impl Default for ShaperPage {
    fn default() -> Self {
        Self::new()
    }
}

impl ShaperPage {
    pub(crate) fn new() -> Self {
        glib::Object::new(&[])
    }

    pub(crate) fn shaperstyle_menubutton(&self) -> MenuButton {
        self.imp().shaperstyle_menubutton.get()
    }

    pub(crate) fn shaperstyle_image(&self) -> Image {
        self.imp().shaperstyle_image.get()
    }

    pub(crate) fn shaperstyle_listbox(&self) -> ListBox {
        self.imp().shaperstyle_listbox.get()
    }

    pub(crate) fn shaperstyle_smooth_row(&self) -> adw::ActionRow {
        self.imp().shaperstyle_smooth_row.get()
    }

    pub(crate) fn shaperstyle_rough_row(&self) -> adw::ActionRow {
        self.imp().shaperstyle_rough_row.get()
    }

    pub(crate) fn shapeconfig_menubutton(&self) -> MenuButton {
        self.imp().shapeconfig_menubutton.get()
    }

    pub(crate) fn width_spinbutton(&self) -> SpinButton {
        self.imp().width_spinbutton.get()
    }

    pub(crate) fn stroke_colorpicker(&self) -> ColorPicker {
        self.imp().stroke_colorpicker.get()
    }

    pub(crate) fn fill_colorpicker(&self) -> ColorPicker {
        self.imp().fill_colorpicker.get()
    }

    pub(crate) fn shapebuildertype_menubutton(&self) -> MenuButton {
        self.imp().shapebuildertype_menubutton.get()
    }

    pub(crate) fn shapebuildertype_image(&self) -> Image {
        self.imp().shapebuildertype_image.get()
    }

    pub(crate) fn shapebuildertype_listbox(&self) -> ListBox {
        self.imp().shapebuildertype_listbox.get()
    }

    pub(crate) fn shapebuildertype_line_row(&self) -> adw::ActionRow {
        self.imp().shapebuildertype_line_row.get()
    }

    pub(crate) fn shapebuildertype_rectangle_row(&self) -> adw::ActionRow {
        self.imp().shapebuildertype_rectangle_row.get()
    }

    pub(crate) fn shapebuildertype_coordsystem2d_row(&self) -> adw::ActionRow {
        self.imp().shapebuildertype_coordsystem2d_row.get()
    }

    pub(crate) fn shapebuildertype_coordsystem3d_row(&self) -> adw::ActionRow {
        self.imp().shapebuildertype_coordsystem3d_row.get()
    }

    pub(crate) fn shapebuildertype_quadrantcoordsystem2d_row(&self) -> adw::ActionRow {
        self.imp().shapebuildertype_quadrantcoordsystem2d_row.get()
    }

    pub(crate) fn shapebuildertype_ellipse_row(&self) -> adw::ActionRow {
        self.imp().shapebuildertype_ellipse_row.get()
    }

    pub(crate) fn shapebuildertype_fociellipse_row(&self) -> adw::ActionRow {
        self.imp().shapebuildertype_fociellipse_row.get()
    }

    pub(crate) fn shapebuildertype_quadbez_row(&self) -> adw::ActionRow {
        self.imp().shapebuildertype_quadbez_row.get()
    }

    pub(crate) fn shapebuildertype_cubbez_row(&self) -> adw::ActionRow {
        self.imp().shapebuildertype_cubbez_row.get()
    }

    pub(crate) fn constraint_menubutton(&self) -> MenuButton {
        self.imp().shapebuildertype_menubutton.get()
    }

    pub(crate) fn roughstyle_fillstyle(&self) -> FillStyle {
        FillStyle::try_from(self.imp().roughstyle_fillstyle_row.get().selected()).unwrap()
    }

    pub(crate) fn set_roughstyle_fillstyle(&self, fill_style: FillStyle) {
        let position = fill_style.to_u32().unwrap();

        self.imp()
            .roughstyle_fillstyle_row
            .get()
            .set_selected(position);
    }

    pub(crate) fn init(&self, appwindow: &RnoteAppWindow) {
        // Width
        self.width_spinbutton().set_increments(0.1, 2.0);
        self.width_spinbutton()
            .set_range(Shaper::STROKE_WIDTH_MIN, Shaper::STROKE_WIDTH_MAX);
        // Must be set after set_range()
        self.width_spinbutton()
            .set_value(Shaper::STROKE_WIDTH_DEFAULT);

        self.width_spinbutton().connect_value_changed(
            clone!(@weak appwindow => move |width_spinbutton| {
                let shaper_style = appwindow.canvas().engine().borrow_mut().penholder.shaper.style;

                match shaper_style {
                    ShaperStyle::Smooth => appwindow.canvas().engine().borrow_mut().penholder.shaper.smooth_options.stroke_width = width_spinbutton.value(),
                    ShaperStyle::Rough => appwindow.canvas().engine().borrow_mut().penholder.shaper.rough_options.stroke_width = width_spinbutton.value(),
                }
            }),
        );

        // Stroke color
        self.stroke_colorpicker().connect_notify_local(
            Some("current-color"),
            clone!(@weak appwindow => move |stroke_colorpicker, _paramspec| {
                let color = stroke_colorpicker.property::<gdk::RGBA>("current-color").into_compose_color();
                let shaper_style = appwindow.canvas().engine().borrow_mut().penholder.shaper.style;

                match shaper_style {
                    ShaperStyle::Smooth => appwindow.canvas().engine().borrow_mut().penholder.shaper.smooth_options.stroke_color = Some(color),
                    ShaperStyle::Rough => appwindow.canvas().engine().borrow_mut().penholder.shaper.rough_options.stroke_color= Some(color),
                }
            }),
        );

        // Fill color
        self.fill_colorpicker().connect_notify_local(
            Some("current-color"),
            clone!(@weak appwindow => move |fill_colorpicker, _paramspec| {
                let color = fill_colorpicker.property::<gdk::RGBA>("current-color").into_compose_color();
                let shaper_style = appwindow.canvas().engine().borrow_mut().penholder.shaper.style;

                match shaper_style {
                    ShaperStyle::Smooth => appwindow.canvas().engine().borrow_mut().penholder.shaper.smooth_options.fill_color = Some(color),
                    ShaperStyle::Rough => appwindow.canvas().engine().borrow_mut().penholder.shaper.rough_options.fill_color= Some(color),
                }
            }),
        );

        // Shaper style
        self.shaperstyle_listbox().connect_row_selected(
            clone!(@weak self as shaperpage, @weak appwindow => move |_shaperstyle_listbox, selected_row| {
                if let Some(selected_row) = selected_row.map(|selected_row| {selected_row.downcast_ref::<adw::ActionRow>().unwrap()}) {
                    {
                        let engine = appwindow.canvas().engine();
                        let engine = &mut *engine.borrow_mut();

                        engine.penholder.shaper.style = ShaperStyle::try_from(selected_row.index() as u32).unwrap_or_default();
                        engine.penholder.shaper.smooth_options.stroke_width = shaperpage.width_spinbutton().value();
                        engine.penholder.shaper.smooth_options.stroke_color = Some(shaperpage.stroke_colorpicker().current_color().into_compose_color());
                        engine.penholder.shaper.smooth_options.fill_color = Some(shaperpage.fill_colorpicker().current_color().into_compose_color());
                        engine.penholder.shaper.rough_options.stroke_width = shaperpage.width_spinbutton().value();
                        engine.penholder.shaper.rough_options.stroke_color = Some(shaperpage.stroke_colorpicker().current_color().into_compose_color());
                        engine.penholder.shaper.rough_options.fill_color = Some(shaperpage.fill_colorpicker().current_color().into_compose_color());
                    }

                    // Need to refresh the whole page, because changing the style affects multiple widgets
                    shaperpage.refresh_ui(&appwindow);
                }
            }),
        );

        // Rough style
        // Fill style
        self.imp().roughstyle_fillstyle_row.get().connect_selected_notify(clone!(@weak self as shaperpage, @weak appwindow => move |_roughstyle_fillstyle_row| {
            appwindow.canvas().engine().borrow_mut().penholder.shaper.rough_options.fill_style = shaperpage.roughstyle_fillstyle();
        }));

        // Hachure angle
        self.imp().roughstyle_hachure_angle_spinbutton.get().connect_value_changed(clone!(@weak self as shaperpage, @weak appwindow => move |spinbutton| {
            appwindow.canvas().engine().borrow_mut().penholder.shaper.rough_options.hachure_angle = spinbutton.value().round().to_radians().clamp(-std::f64::consts::PI, std::f64::consts::PI);
        }));

        // Constraints
        self.imp()
            .constraint_enabled_switch
            .get()
            .connect_state_notify(clone!(@weak appwindow => move |switch|  {
                appwindow.canvas().engine().borrow_mut().penholder.shaper.constraints.enabled = switch.state();
            }));

        self.imp()
            .constraint_one_to_one_switch
            .get()
            .connect_state_notify(clone!(@weak appwindow => move |switch|  {
                if switch.state() {
                    appwindow.canvas().engine().borrow_mut().penholder.shaper.constraints.ratios.insert(ConstraintRatio::OneToOne);
                } else {
                    appwindow.canvas().engine().borrow_mut().penholder.shaper.constraints.ratios.remove(&ConstraintRatio::OneToOne);
                }
            }));

        self.imp()
            .constraint_three_to_two_switch
            .get()
            .connect_state_notify(clone!(@weak appwindow => move |switch|  {
                if switch.state() {
                    appwindow.canvas().engine().borrow_mut().penholder.shaper.constraints.ratios.insert(ConstraintRatio::ThreeToTwo);
                } else {
                    appwindow.canvas().engine().borrow_mut().penholder.shaper.constraints.ratios.remove(&ConstraintRatio::ThreeToTwo);
                }
            }));

        self.imp()
            .constraint_golden_switch
            .get()
            .connect_state_notify(clone!(@weak appwindow => move |switch|  {
                if switch.state() {
                    appwindow.canvas().engine().borrow_mut().penholder.shaper.constraints.ratios.insert(ConstraintRatio::Golden);
                } else {
                    appwindow.canvas().engine().borrow_mut().penholder.shaper.constraints.ratios.remove(&ConstraintRatio::Golden);
                }
            }));

        // shape builder type
        self.shapebuildertype_listbox().connect_row_selected(
            clone!(@weak self as shaperpage, @weak appwindow => move |_shapetype_listbox, selected_row| {
                if let Some(selected_row) = selected_row.map(|selected_row| {selected_row.downcast_ref::<adw::ActionRow>().unwrap()}) {
                    appwindow.canvas().engine().borrow_mut().penholder.shaper.builder_type = ShapeBuilderType::try_from(selected_row.index() as u32).unwrap_or_default();

                    // Need to refresh the whole page, because changing the builder type affects multiple widgets
                    shaperpage.refresh_ui(&appwindow);
                }
            }),
        );
    }

    pub(crate) fn refresh_ui(&self, appwindow: &RnoteAppWindow) {
        let shaper = appwindow
            .canvas()
            .engine()
            .borrow()
            .penholder
            .shaper
            .clone();

        match shaper.style {
            ShaperStyle::Smooth => {
                self.shaperstyle_listbox()
                    .select_row(Some(&self.shaperstyle_smooth_row()));
                self.width_spinbutton()
                    .set_value(shaper.smooth_options.stroke_width);
                self.stroke_colorpicker()
                    .set_current_color(gdk::RGBA::from_compose_color(
                        shaper
                            .smooth_options
                            .stroke_color
                            .unwrap_or(Color::TRANSPARENT),
                    ));
                self.fill_colorpicker()
                    .set_current_color(gdk::RGBA::from_compose_color(
                        shaper
                            .smooth_options
                            .fill_color
                            .unwrap_or(Color::TRANSPARENT),
                    ));
                self.shaperstyle_image()
                    .set_icon_name(Some("pen-shaper-style-smooth-symbolic"));
            }
            ShaperStyle::Rough => {
                self.shaperstyle_listbox()
                    .select_row(Some(&self.shaperstyle_rough_row()));
                self.width_spinbutton()
                    .set_value(shaper.rough_options.stroke_width);
                self.stroke_colorpicker()
                    .set_current_color(gdk::RGBA::from_compose_color(
                        shaper
                            .rough_options
                            .stroke_color
                            .unwrap_or(Color::TRANSPARENT),
                    ));
                self.fill_colorpicker()
                    .set_current_color(gdk::RGBA::from_compose_color(
                        shaper
                            .rough_options
                            .fill_color
                            .unwrap_or(Color::TRANSPARENT),
                    ));
                self.shaperstyle_image()
                    .set_icon_name(Some("pen-shaper-style-rough-symbolic"));
            }
        }

        // Rough style
        self.set_roughstyle_fillstyle(shaper.rough_options.fill_style);
        self.imp()
            .roughstyle_hachure_angle_spinbutton
            .set_value(shaper.rough_options.hachure_angle.to_degrees());

        // constraints
        self.imp()
            .constraint_enabled_switch
            .set_state(shaper.constraints.enabled);
        self.imp().constraint_one_to_one_switch.set_state(
            shaper
                .constraints
                .ratios
                .get(&ConstraintRatio::OneToOne)
                .is_some(),
        );
        self.imp().constraint_three_to_two_switch.set_state(
            shaper
                .constraints
                .ratios
                .get(&ConstraintRatio::ThreeToTwo)
                .is_some(),
        );
        self.imp().constraint_golden_switch.set_state(
            shaper
                .constraints
                .ratios
                .get(&ConstraintRatio::Golden)
                .is_some(),
        );

        // builder type
        match shaper.builder_type {
            ShapeBuilderType::Line => {
                self.shapebuildertype_listbox().select_row(Some(
                    &appwindow
                        .penssidebar()
                        .shaper_page()
                        .shapebuildertype_line_row(),
                ));
                self.shapebuildertype_image()
                    .set_icon_name(Some("shape-line-symbolic"));
            }
            ShapeBuilderType::Rectangle => {
                self.shapebuildertype_listbox().select_row(Some(
                    &appwindow
                        .penssidebar()
                        .shaper_page()
                        .shapebuildertype_rectangle_row(),
                ));
                self.shapebuildertype_image()
                    .set_icon_name(Some("shape-rectangle-symbolic"));
            }
            ShapeBuilderType::CoordSystem2D => {
                self.shapebuildertype_listbox()
                    .select_row(Some(&self.shapebuildertype_coordsystem2d_row()));
                self.shapebuildertype_image()
                    .set_icon_name(Some("shape-coordsystem2d-symbolic"));
            }
            ShapeBuilderType::CoordSystem3D => {
                self.shapebuildertype_listbox()
                    .select_row(Some(&self.shapebuildertype_coordsystem3d_row()));
                self.shapebuildertype_image()
                    .set_icon_name(Some("shape-coordsystem3d-symbolic"));
            }
            ShapeBuilderType::QuadrantCoordSystem2D => {
                self.shapebuildertype_listbox()
                    .select_row(Some(&self.shapebuildertype_quadrantcoordsystem2d_row()));
                self.shapebuildertype_image()
                    .set_icon_name(Some("shape-quadrantcoordsystem2d-symbolic"));
            }
            ShapeBuilderType::Ellipse => {
                self.shapebuildertype_listbox()
                    .select_row(Some(&self.shapebuildertype_ellipse_row()));
                self.shapebuildertype_image()
                    .set_icon_name(Some("shape-ellipse-symbolic"));
            }
            ShapeBuilderType::FociEllipse => {
                self.shapebuildertype_listbox()
                    .select_row(Some(&self.shapebuildertype_fociellipse_row()));
                self.shapebuildertype_image()
                    .set_icon_name(Some("shape-fociellipse-symbolic"));
            }
            ShapeBuilderType::QuadBez => {
                self.shapebuildertype_listbox().select_row(Some(
                    &appwindow
                        .penssidebar()
                        .shaper_page()
                        .shapebuildertype_quadbez_row(),
                ));
                self.shapebuildertype_image()
                    .set_icon_name(Some("shape-quadbez-symbolic"));
            }
            ShapeBuilderType::CubBez => {
                self.shapebuildertype_listbox().select_row(Some(
                    &appwindow
                        .penssidebar()
                        .shaper_page()
                        .shapebuildertype_cubbez_row(),
                ));
                self.shapebuildertype_image()
                    .set_icon_name(Some("shape-cubbez-symbolic"));
            }
        }
    }
}
