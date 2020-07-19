use gtk::prelude::*;
use gio::prelude::*;
use gtk::{Application, ApplicationWindow, Button, Builder};
use std::env::args;

fn main() {
    let application: Application = Application::new(
        Some("com.parkerokonek.paradoxmerger"),
        Default::default(),
    ).expect("Failed to initialize GTK application.");

    application.connect_activate(|app| {build_ui(app);});

    application.run(&args().collect::<Vec<_>>());
}

fn build_ui(application: &gtk::Application) {
    let glade_src = include_str!("gui_layout.glade");
    let builder = Builder::from_string(glade_src);
    let window: gtk::Window = builder.get_object("window").expect("Window failed to initialize.");

    window.set_application(Some(application));
    
    window.show_all();
}