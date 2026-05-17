use gtk4::prelude::*;
use gtk4::{Application, ApplicationWindow, Box, Label, Orientation};

pub struct UiBuilder {
    pub window: ApplicationWindow,
}

impl UiBuilder {
    pub fn new(app: &Application) -> Self {
        let window = ApplicationWindow::builder()
            .application(app)
            .default_width(1000)
            .default_height(200)
            .decorated(false)
            .resizable(false)
            .modal(true)
            .title("Hello world!!!!")
            .build();

        let root = Box::new(Orientation::Vertical, 8);

        root.append(&Self::build_header());

        window.set_child(Some(&root));
        Self { window }
    }

    fn build_header() -> Box {
        let header = Box::new(Orientation::Vertical, 0);

        let shutdown_header = Label::builder().label("Shutting down...").build();

        header.append(&shutdown_header);

        header
    }
}
