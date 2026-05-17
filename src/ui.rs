use gtk4::{Application, ApplicationWindow, Box, Button, Label, ListBoxRow, Orientation};
use gtk4::{ListBox, prelude::*};

pub struct UiBuilder {
    pub window: ApplicationWindow,
}

impl UiBuilder {
    pub fn new(app: &Application) -> Self {
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

        root.append(&Self::build_header(2));
        root.append(&Self::build_app_list());
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
            .build();

        header.append(&shutdown_header);
        header
    }

    fn build_app_list() -> ListBox {
        // .vexpand will push the footer to the bottom of the window
        let list = ListBox::builder().vexpand(true).build();

        // Hardcode list of apps for now
        let apps = ["kitty", "spotify"];

        for app in apps {
            let row = ListBoxRow::builder()
                .activatable(false)
                .can_focus(false)
                .build();

            let row_box = Box::new(Orientation::Vertical, 8);
            let name_label = Label::builder().label(app).build();
            row_box.append(&name_label);

            row.set_child(Some(&row_box));

            list.append(&row);
        }

        list
    }

    fn build_footer() -> Box {
        let footer = Box::new(Orientation::Horizontal, 8);

        let force_quit_btn = Button::builder().label("Force quit anyway").build();
        let cancel_btn = Button::builder().label("Cancel").build();

        footer.append(&force_quit_btn);
        footer.append(&cancel_btn);

        footer
    }
}
