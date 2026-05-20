use gtk4::{Align, Application, ApplicationWindow, Box, Button, Label, ListBoxRow, Orientation};
use gtk4::{ListBox, prelude::*};

use crate::app::AppState;
use crate::client::WaylandClient;

pub struct UiBuilder {
    pub window: ApplicationWindow,
    app_list: ListBox,
    header: Box,
}

impl UiBuilder {
    pub fn new<T: WaylandClient>(app: &Application, state: &AppState<T>) -> Self {
        Self::load_css();

        let window = ApplicationWindow::builder()
            .application(app)
            .default_width(1000)
            .default_height(500)
            .decorated(false)
            .resizable(false)
            .modal(true)
            .title("gtkshutdown")
            .build();

        let root = Box::new(Orientation::Vertical, 8);
        let app_list = Self::build_app_list(state);
        let header = Self::build_header(state.get_num_clients());

        root.append(&header);
        root.append(&app_list);
        root.append(&Self::build_footer());

        window.set_child(Some(&root));
        Self {
            window,
            app_list,
            header,
        }
    }

    pub fn update<T: WaylandClient>(&mut self, state: &AppState<T>) {
        Self::update_app_list(&self.app_list, state);
        Self::update_header(&self.header, state.get_num_clients());
    }

    fn load_css() {
        let provider = gtk4::CssProvider::new();
        provider.load_from_data(include_str!("assets/style.css"));

        gtk4::style_context_add_provider_for_display(
            &gtk4::gdk::Display::default().expect("Could not connect to a display."),
            &provider,
            gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
    }

    fn build_header(num_apps: usize) -> Box {
        let header = Box::new(Orientation::Vertical, 0);
        Self::update_header(&header, num_apps);

        header
    }

    fn update_header(header: &Box, num_apps: usize) {
        // Clear header
        while let Some(row) = header.first_child() {
            header.remove(&row);
        }

        let shutdown_header = Label::builder()
            .label(format!("Closing {num_apps} apps..."))
            .css_classes(["title"])
            .build();

        header.append(&shutdown_header);
    }

    fn build_app_list<T: WaylandClient>(state: &AppState<T>) -> ListBox {
        let list = ListBox::builder()
            // .vexpand will push the footer to the bottom of the window
            .vexpand(true)
            .focus_on_click(false)
            .focusable(false)
            .css_classes(["app-list"])
            .selection_mode(gtk4::SelectionMode::None)
            .build();

        Self::update_app_list(&list, state);
        list
    }

    fn update_app_list<T: WaylandClient>(list: &ListBox, state: &AppState<T>) {
        // Clear list
        while let Some(row) = list.first_child() {
            list.remove(&row);
        }

        // Repopulate
        for client in &state.clients {
            // Don't display the shutdown app
            if client.app_id() == crate::APP_ID {
                continue;
            }

            let row = ListBoxRow::builder()
                .activatable(false)
                .can_focus(false)
                .halign(Align::Start)
                .build();

            let row_box = Box::new(Orientation::Vertical, 8);

            let app_id_label = Label::builder()
                .halign(Align::Start)
                .css_classes(["app-id"])
                .label(client.app_id())
                .build();
            row_box.append(&app_id_label);

            let title = client.title().unwrap_or("");
            let title_label = Label::builder()
                .halign(Align::Start)
                .css_classes(["app-title"])
                .label(title)
                .ellipsize(gtk4::pango::EllipsizeMode::End)
                .max_width_chars(1000)
                .build();
            row_box.append(&title_label);

            row.set_child(Some(&row_box));
            list.append(&row);
        }
    }

    fn build_footer() -> Box {
        let footer = Box::builder()
            .orientation(Orientation::Horizontal)
            .spacing(8)
            .halign(Align::End)
            .css_classes(["footer"])
            .build();

        let force_quit_btn = Button::builder().label("Force quit anyway").build();
        let cancel_btn = Button::builder().label("Cancel").build();

        footer.append(&force_quit_btn);
        footer.append(&cancel_btn);

        footer
    }
}
