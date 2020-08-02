use gtk::prelude::*;
use gio::prelude::*;
use gtk::{Application, ApplicationWindow, Button, Builder};
use std::env::args;

use paradoxmerger::{ArgOptions,ConfigOptions,ModPack,ModInfo};

fn main() {
    let application: Application = Application::new(
        Some("com.parkerokonek.paradoxmerger"),
        Default::default(),
    ).expect("Failed to initialize GTK application.");

    let arg_options = ArgOptions {
        config_path: ["./merger.toml"].iter().collect(), 
        extract: false, 
        dry_run: true, 
        verbose: false, 
        game_id: "CK2".to_string(), 
        patch_name: "merger_patch".to_owned()
    };
    
    let conf_options = ConfigOptions {
        game_name: "CK2".to_owned(), 
        mod_path: ["/home/parker/.paradoxinteractive/Crusader Kings II"].iter().collect(), 
        data_path: ["/home/parker/.steam/steam/steamapps/common/Crusader Kings II"].iter().collect(), 
        valid_paths: ["history", "common", "decisions", "events", "localisation"].iter().map(|x| [x].iter().collect()).collect()
    };

    application.connect_activate(move |app| {
        build_ui(app,&arg_options,&conf_options);
    });

    application.run(&args().collect::<Vec<_>>());
}

fn build_ui(application: &gtk::Application, arg_options: &ArgOptions, conf_options: &ConfigOptions) {
    let glade_src = include_str!("gui_layout.glade");
    let builder = Builder::from_string(glade_src);
    let window: gtk::Window = builder.get_object("window").expect("Window failed to initialize.");

    let button_scan: Button = builder.get_object("button_scan").expect("Couldn't get scan button.");

    button_scan.connect_clicked(|_| {
        //let mut mod_pack = ModPack::new().restrict_paths(&conf_options.valid_paths);
        
    });

    window.set_application(Some(application));
    
    window.show_all();
}
