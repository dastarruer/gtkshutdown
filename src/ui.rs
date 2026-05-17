use gtk4::prelude::*;
use gtk4::{Application, ApplicationWindow, Box, Label, Orientation};

pub struct UiBuilder {
    pub window: ApplicationWindow,
}

impl UiBuilder {
    pub fn new(app: &Application) -> Self {
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

        root.append(&Self::build_header());
        root.append(&Self::build_app_list());

        window.set_child(Some(&root));
        Self { window }
    }

    fn build_header() -> Box {
        let header = Box::new(Orientation::Vertical, 0);

        let shutdown_header = Label::builder().label("Shutting down...").build();

        header.append(&shutdown_header);

        header
    }

    fn build_app_list() -> ListBox {
        let list = ListBox::new();

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
}
