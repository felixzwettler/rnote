mod workspacelist;
mod workspacelistentry;
mod workspacerow;

use gtk4::ConstantExpression;
// Re-exports
pub(crate) use workspacelist::WorkspaceList;
pub(crate) use workspacelistentry::WorkspaceListEntry;
pub(crate) use workspacerow::WorkspaceRow;

// Imports
use crate::appwindow::RnoteAppWindow;
use crate::dialogs;

use gtk4::{
    gdk, gio, glib, glib::clone, prelude::*, subclass::prelude::*, Button, CompositeTemplate,
    ListBox, ScrolledWindow, Widget,
};
use std::path::PathBuf;

mod imp {
    use super::*;

    #[derive(Debug, CompositeTemplate)]
    #[template(resource = "/com/github/flxzt/rnote/ui/workspacesbar.ui")]
    pub(crate) struct WorkspacesBar {
        pub(crate) action_group: gio::SimpleActionGroup,
        pub(crate) workspace_list: WorkspaceList,

        #[template_child]
        pub(crate) workspaces_scroller: TemplateChild<ScrolledWindow>,
        #[template_child]
        pub(crate) workspaces_listbox: TemplateChild<ListBox>,
        #[template_child]
        pub(crate) add_workspace_button: TemplateChild<Button>,
        #[template_child]
        pub(crate) remove_selected_workspace_button: TemplateChild<Button>,
        #[template_child]
        pub(crate) edit_selected_workspace_button: TemplateChild<Button>,
    }

    impl Default for WorkspacesBar {
        fn default() -> Self {
            Self {
                action_group: gio::SimpleActionGroup::new(),
                workspace_list: WorkspaceList::default(),

                workspaces_scroller: Default::default(),
                workspaces_listbox: Default::default(),
                add_workspace_button: Default::default(),
                remove_selected_workspace_button: Default::default(),
                edit_selected_workspace_button: Default::default(),
            }
        }
    }

    #[glib::object_subclass]
    impl ObjectSubclass for WorkspacesBar {
        const NAME: &'static str = "WorkspacesBar";
        type Type = super::WorkspacesBar;
        type ParentType = Widget;

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for WorkspacesBar {
        fn constructed(&self) {
            self.parent_constructed();

            self.instance()
                .insert_action_group("workspacesbar", Some(&self.action_group));
        }

        fn dispose(&self) {
            while let Some(child) = self.instance().first_child() {
                child.unparent();
            }
        }
    }

    impl WidgetImpl for WorkspacesBar {}
}

glib::wrapper! {
    pub(crate) struct WorkspacesBar(ObjectSubclass<imp::WorkspacesBar>)
        @extends gtk4::Widget;
}

impl Default for WorkspacesBar {
    fn default() -> Self {
        Self::new()
    }
}

impl WorkspacesBar {
    pub(crate) fn new() -> Self {
        glib::Object::new(&[])
    }

    pub(crate) fn action_group(&self) -> gio::SimpleActionGroup {
        self.imp().action_group.clone()
    }

    pub(crate) fn workspaces_scroller(&self) -> ScrolledWindow {
        self.imp().workspaces_scroller.clone()
    }

    pub(crate) fn push_workspace(&self, entry: WorkspaceListEntry) {
        self.imp().workspace_list.push(entry);

        let n_items = self.imp().workspace_list.n_items();
        self.select_workspace_by_index(n_items.saturating_sub(1));
    }

    pub(crate) fn remove_selected_workspace(&self) {
        let n_items = self.imp().workspace_list.n_items();

        // never remove the last row
        if n_items > 0 {
            let i = self
                .selected_workspace_index()
                .unwrap_or_else(|| n_items.saturating_sub(1));

            self.imp().workspace_list.remove(i as usize);

            self.select_workspace_by_index(i);
        }
    }

    pub(crate) fn select_workspace_by_index(&self, index: u32) {
        let n_items = self.imp().workspace_list.n_items();

        self.imp().workspaces_listbox.select_row(
            self.imp()
                .workspaces_listbox
                .row_at_index(index.min(n_items.saturating_sub(1)) as i32)
                .as_ref(),
        );
    }

    pub(crate) fn selected_workspace_index(&self) -> Option<u32> {
        self.imp()
            .workspaces_listbox
            .selected_row()
            .map(|r| r.index() as u32)
    }

    pub(crate) fn selected_workspacelistentry(&self) -> Option<WorkspaceListEntry> {
        self.selected_workspace_index().and_then(|i| {
            self.imp()
                .workspace_list
                .item(i)
                .map(|o| o.downcast::<WorkspaceListEntry>().unwrap())
        })
    }

    pub(crate) fn replace_selected_workspacelistentry(&self, entry: WorkspaceListEntry) {
        if let Some(i) = self.selected_workspace_index() {
            self.imp().workspace_list.replace(i as usize, entry);

            self.select_workspace_by_index(i);
        }
    }

    #[allow(unused)]
    pub(crate) fn set_selected_workspace_dir(&self, dir: PathBuf) {
        if let Some(i) = self.selected_workspace_index() {
            let row = self.imp().workspace_list.remove(i as usize);
            row.set_dir(dir.to_string_lossy().into());
            self.imp().workspace_list.insert(i as usize, row);

            self.select_workspace_by_index(i);
        }
    }

    #[allow(unused)]
    pub(crate) fn set_selected_workspace_icon(&self, icon: String) {
        if let Some(i) = self.selected_workspace_index() {
            let row = self.imp().workspace_list.remove(i as usize);
            row.set_icon(icon);
            self.imp().workspace_list.insert(i as usize, row);

            self.select_workspace_by_index(i);
        }
    }

    #[allow(unused)]
    pub(crate) fn set_selected_workspace_color(&self, color: gdk::RGBA) {
        if let Some(i) = self.selected_workspace_index() {
            let row = self.imp().workspace_list.remove(i as usize);
            row.set_color(color);
            self.imp().workspace_list.insert(i as usize, row);

            self.select_workspace_by_index(i);
        }
    }

    #[allow(unused)]
    pub(crate) fn set_selected_workspace_name(&self, name: String) {
        if let Some(i) = self.selected_workspace_index() {
            let row = self.imp().workspace_list.remove(i as usize);
            row.set_name(name);
            self.imp().workspace_list.insert(i as usize, row);

            self.select_workspace_by_index(i);
        }
    }

    pub(crate) fn save_to_settings(&self, settings: &gio::Settings) {
        if let Err(e) = settings.set("workspace-list", &self.imp().workspace_list) {
            log::error!("saving `workspace-list` to settings failed with Err: {e:?}");
        }

        if let Err(e) = settings.set(
            "selected-workspace-index",
            &self.selected_workspace_index().unwrap_or(0),
        ) {
            log::error!("saving `selected-workspace-index` to settings failed with Err: {e:?}");
        }
    }

    pub(crate) fn load_from_settings(&self, settings: &gio::Settings) {
        let workspace_list = settings.get::<WorkspaceList>("workspace-list");
        // Be sure to get the index before loading the workspaces, else the setting gets overridden
        let selected_workspace_index = settings.uint("selected-workspace-index");

        self.imp().workspace_list.replace_self(workspace_list);

        self.select_workspace_by_index(selected_workspace_index);
    }

    pub(crate) fn init(&self, appwindow: &RnoteAppWindow) {
        self.setup_actions(appwindow);

        self.imp().workspace_list.connect_items_changed(
            clone!(@weak self as workspacesbar, @weak appwindow => move |list, _, _, _| {
                workspacesbar.imp().remove_selected_workspace_button.get().set_sensitive(list.n_items() > 1);
                workspacesbar.imp().edit_selected_workspace_button.get().set_sensitive(list.n_items() > 0);

                workspacesbar.save_to_settings(&appwindow.app_settings());
            }),
        );

        let workspace_listbox = self.imp().workspaces_listbox.get();
        workspace_listbox.connect_selected_rows_changed(
            clone!(@weak appwindow, @weak self as workspacesbar => move |_| {
                if let Some(dir) = workspacesbar.selected_workspacelistentry().map(|e| e.dir()) {
                     appwindow.workspacebrowser().set_dirlist_file(Some(&gio::File::for_path(dir)));

                    workspacesbar.save_to_settings(&appwindow.app_settings());
                }

            }),
        );

        workspace_listbox.bind_model(
            Some(&self.imp().workspace_list),
            clone!(@strong appwindow => move |obj| {
                let entry = obj.to_owned().downcast::<WorkspaceListEntry>().unwrap();
                let workspace_row = WorkspaceRow::new(&entry);
                workspace_row.init(&appwindow);

                let entry_expr = ConstantExpression::new(&entry);
                entry_expr.bind(&workspace_row, "entry", None::<&glib::Object>);

                workspace_row.upcast::<Widget>()
            }),
        );

        self.imp().add_workspace_button.get().connect_clicked(
            clone!(@weak self as workspacesbar, @weak appwindow => move |_| {
                adw::prelude::ActionGroupExt::activate_action(&workspacesbar.action_group(), "add-workspace", None);
            }));

        self.imp().remove_selected_workspace_button.get().connect_clicked(
            clone!(@weak self as workspacesbar, @weak appwindow => move |_| {
                adw::prelude::ActionGroupExt::activate_action(&workspacesbar.action_group(), "remove-selected-workspace", None);
            }));

        self.imp().edit_selected_workspace_button.get().connect_clicked(
            clone!(@weak self as workspacesbar, @weak appwindow => move |_| {
                adw::prelude::ActionGroupExt::activate_action(&workspacesbar.action_group(), "edit-selected-workspace", None);
            }));
    }

    fn setup_actions(&self, appwindow: &RnoteAppWindow) {
        let imp = self.imp();

        let action_add_workspace = gio::SimpleAction::new("add-workspace", None);
        imp.action_group.add_action(&action_add_workspace);
        let action_remove_selected_workspace =
            gio::SimpleAction::new("remove-selected-workspace", None);
        imp.action_group
            .add_action(&action_remove_selected_workspace);
        let action_edit_selected_workspace =
            gio::SimpleAction::new("edit-selected-workspace", None);
        imp.action_group.add_action(&action_edit_selected_workspace);

        // Add workspace
        action_add_workspace.connect_activate(clone!(@weak self as workspacesbar, @weak appwindow => move |_, _| {
                let entry = workspacesbar.selected_workspacelistentry().unwrap_or_else(|| WorkspaceListEntry::default());
                workspacesbar.push_workspace(entry);

                // Popup the edit dialog after creation
                dialogs::dialog_edit_selected_workspace(&appwindow);
        }));

        // Remove selected workspace
        action_remove_selected_workspace.connect_activate(
            clone!(@weak self as workspacesbar, @weak appwindow => move |_, _| {
                    workspacesbar.remove_selected_workspace();
            }),
        );

        // Edit selected workspace
        action_edit_selected_workspace.connect_activate(clone!(@weak appwindow => move |_, _| {
            dialogs::dialog_edit_selected_workspace(&appwindow);
        }));
    }
}
