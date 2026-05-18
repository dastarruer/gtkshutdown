use gtk4::{Align, Application, ApplicationWindow, Box, Button, Label, ListBoxRow, Orientation};
use gtk4::{ListBox, prelude::*};

use crate::app_state::AppState;

pub struct UiBuilder {
    pub window: ApplicationWindow,
}

impl UiBuilder {
    pub fn new(app: &Application, state: AppState) -> Self {
        Self::load_css();

        let window = ApplicationWindow::builder()
            .application(app)
            .default_width(600)
            .default_height(500)
            .decorated(false)
            .resizable(false)
            .modal(true)
            .title("gtkshutdown")
            .build();

        let root = Box::new(Orientation::Vertical, 8);

        root.append(&Self::build_header(state.clients.iter().count() as i8));
        root.append(&Self::build_app_list(state));
        root.append(&Self::build_footer());

        window.set_child(Some(&root));
        Self { window }
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

    fn build_header(num_apps: i8) -> Box {
        let header = Box::new(Orientation::Vertical, 0);

        let shutdown_header = Label::builder()
            .label(format!("Closing {num_apps} apps..."))
            .css_classes(["title"])
            .build();

        header.append(&shutdown_header);
        header
    }

    fn build_app_list(state: AppState) -> ListBox {
        let list = ListBox::builder()
            // .vexpand will push the footer to the bottom of the window
            .vexpand(true)
            .focus_on_click(false)
            .focusable(false)
            .css_classes(["app-list"])
            .selection_mode(gtk4::SelectionMode::None)
            .build();

        for client in state.clients {
            let row = ListBoxRow::builder()
                .activatable(false)
                .can_focus(false)
                .halign(Align::Start)
                .build();

            let row_box = Box::new(Orientation::Vertical, 8);

            let class_label = Label::builder().label(client.class).build();
            row_box.append(&class_label);

            row.set_child(Some(&row_box));

            list.append(&row);
        }

        list
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
