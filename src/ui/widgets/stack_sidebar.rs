use std::rc::Rc;

use adw::{prelude::*, subclass::prelude::*};
use gtk::glib::{self, clone};

use crate::utils::settings::{SidebarMeterType, SETTINGS};

use super::stack_sidebar_item::ResStackSidebarItem;

mod imp {
    use std::{
        cell::{Cell, RefCell},
        collections::HashMap,
        rc::Rc,
    };

    use super::*;

    use gtk::{gio, CompositeTemplate, SingleSelection};

    #[derive(CompositeTemplate)]
    #[template(resource = "/net/nokyan/Resources/ui/widgets/stack_sidebar.ui")]
    pub struct ResStackSidebar {
        #[template_child]
        pub list_box: TemplateChild<gtk::ListBox>,

        pub stack: RefCell<Rc<gtk::Stack>>,
        pub pages: RefCell<Rc<gtk::SelectionModel>>,

        pub rows: RefCell<HashMap<gtk::ListBoxRow, gtk::StackPage>>,

        pub populating: Cell<bool>,
    }

    impl Default for ResStackSidebar {
        fn default() -> Self {
            Self {
                list_box: Default::default(),
                stack: Default::default(),
                pages: RefCell::new(Rc::new(SingleSelection::new(None::<gio::ListStore>).into())),
                rows: Default::default(),
                populating: Default::default(),
            }
        }
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ResStackSidebar {
        const NAME: &'static str = "ResStackSidebar";
        type Type = super::ResStackSidebar;
        type ParentType = adw::Bin;

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);
        }

        // You must call `Widget`'s `init_template()` within `instance_init()`.
        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for ResStackSidebar {
        fn constructed(&self) {
            self.parent_constructed();
        }
    }

    impl WidgetImpl for ResStackSidebar {}

    impl BinImpl for ResStackSidebar {}

    impl ListBoxRowImpl for ResStackSidebar {}

    impl PreferencesRowImpl for ResStackSidebar {}
}

glib::wrapper! {
    pub struct ResStackSidebar(ObjectSubclass<imp::ResStackSidebar>)
        @extends gtk::Widget;
}

impl ResStackSidebar {
    pub fn new() -> Self {
        glib::Object::new::<Self>()
    }

    pub fn stack(&self) -> Rc<gtk::Stack> {
        self.imp().stack.borrow().clone()
    }

    fn clear(&self) {
        let imp = self.imp();
        for item in imp.rows.borrow().keys() {
            imp.list_box.remove(item);
        }
        imp.rows.borrow_mut().clear();
    }

    fn populate_list(&self) {
        let imp = self.imp();
        imp.populating.set(true);

        let pages = imp.stack.borrow().pages();

        let children = pages.iter::<gtk::StackPage>().flatten().map(|p| {
            p.child()
                .downcast::<adw::ToolbarView>()
                .unwrap()
                .content()
                .unwrap()
        });

        for c in children {
            println!(
                "{:?} - {:?}",
                c,
                c.downcast_ref::<crate::ui::pages::network::ResNetwork>()
                    .is_some()
            );
        }

        for page in imp
            .stack
            .borrow()
            .pages()
            .iter::<gtk::StackPage>()
            .flatten()
        {
            let child = page
                .child()
                .downcast::<adw::ToolbarView>()
                .unwrap()
                .content()
                .unwrap();

            let sidebar_item = ResStackSidebarItem::new(
                child.property("tab_name"),
                child.property("icon"),
                child.property("tab_subtitle"),
            );

            child
                .bind_property("tab_name", &sidebar_item, "name")
                .sync_create()
                .build();

            child
                .bind_property("icon", &sidebar_item, "icon")
                .sync_create()
                .build();

            child
                .bind_property("tab_subtitle", &sidebar_item, "subtitle")
                .sync_create()
                .build();

            sidebar_item.set_subtitle_visible(SETTINGS.sidebar_details());
            SETTINGS.connect_sidebar_details(
                clone!(@strong sidebar_item as item => move |sidebar_details| {
                    item.set_subtitle_visible(sidebar_details);
                }),
            );

            // TODO: generalize to "uses_meter"?
            if !child.property::<bool>("uses_progress_bar") {
                sidebar_item.set_progress_bar_visible(false);
                sidebar_item.set_graph_visible(false);
            } else {
                if child.has_property("main_graph_color", Some(glib::Bytes::static_type())) {
                    let b = child.property::<glib::Bytes>("main_graph_color");
                    sidebar_item.set_graph_color(b[0], b[1], b[2]);
                }

                sidebar_item.set_progress_bar_visible(
                    SETTINGS.sidebar_meter_type() == SidebarMeterType::ProgressBar,
                );
                sidebar_item
                    .set_graph_visible(SETTINGS.sidebar_meter_type() == SidebarMeterType::Graph);
                SETTINGS.connect_sidebar_meter_type(
                    clone!(@strong sidebar_item as item => move |sidebar_meter_type| {
                        item.set_progress_bar_visible(sidebar_meter_type == SidebarMeterType::ProgressBar);
                        item.set_graph_visible(sidebar_meter_type == SidebarMeterType::Graph);
                    }),
                );
                child
                    .bind_property("usage", &sidebar_item, "usage")
                    .sync_create()
                    .build();
            }

            let row = gtk::ListBoxRow::builder()
                .child(&sidebar_item)
                .selectable(true)
                .can_focus(true)
                .can_target(true)
                .build();

            imp.list_box.append(&row);

            if let Some(visible_page) = imp.stack.borrow().visible_child() {
                if visible_page == page.child() {
                    imp.list_box.select_row(Some(&row));
                }
            }

            imp.rows.borrow_mut().insert(row, page);
        }

        imp.populating.set(false);
    }

    pub fn set_stack(&self, stack: &gtk::Stack) {
        let imp = self.imp();

        *imp.stack.borrow_mut() = Rc::from(stack.clone());
        *imp.pages.borrow_mut() = Rc::from(stack.clone().pages());

        imp.pages.borrow().connect_items_changed(
            clone!(@strong self as this => move |_, _, _, _| {
                this.clear();
                this.populate_list();
            }),
        );

        imp.list_box.connect_selected_rows_changed(
            clone!(@strong self as this => move |list_box| {
                let imp = this.imp();
                if let Some(selected) = list_box.selected_row() {
                    if !imp.populating.get() {
                        imp.stack.borrow().set_visible_child(&imp.rows.borrow().get(&selected).unwrap().child());
                    }
                }
            }),
        );
    }
}
