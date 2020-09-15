#![recursion_limit="256"]
mod vgtk_ext;

use vgtk::ext::*;
use vgtk::lib::gio::ApplicationFlags;
use vgtk::lib::gtk::*;
use vgtk::{gtk, run, Component, UpdateAction, VNode};

use vgtk_ext::*;

use paradoxmerger::configs::{ConfigOptions,fetch_user_configs};

#[derive(Clone, Debug)]
struct Model {
    configs: Vec<ConfigOptions>
}

impl Default for Model {
    fn default() -> Self {
        Self {
            configs: fetch_user_configs(true).unwrap_or(Vec::new()),
        }
    }
}

#[derive(Clone, Debug)]
enum Message {
    Exit,
}

impl Component for Model {
    type Message = Message;
    type Properties = ();

    fn update(&mut self, msg: Self::Message) -> UpdateAction<Self> {
        match msg {
            Message::Exit => {
                vgtk::quit();
                UpdateAction::None
            }
        }
    }

    fn view(&self) -> VNode<Model> {
        gtk! {
            <Application::new_unwrap(Some("com.example.paradoxmerger"), ApplicationFlags::empty())>
                <Window border_width=20 on destroy=|_| Message::Exit>
                <Box>
                <ListBox>
                <ListBoxRow>
                <Label label="I am an example mod".to_owned() />
                </ListBoxRow>
                </ListBox>
                <Box orientation=Orientation::Vertical>
                <Box>
                    <ComboBoxText items=list_config_entries(&self.configs) tooltip_text="Select a game to patch.".to_owned() />
                    <Button label="+".to_owned() tooltip_text="Modify game entries.".to_owned() />
                </Box>
                <Box>
                    <Entry />
                    <CheckButton />
                </Box>
                <Box>
                    <Button label="Scan".to_owned()/>
                    <Button label="Patch".to_owned()/>
                </Box>
                </Box>
                </Box>
                </Window>
            </Application>
        }
    }
}

fn list_config_entries(configs: &[ConfigOptions]) -> Vec<(Option<String>,String)> {
    let mut vec = Vec::new();
    for conf in configs {
        vec.push((None,conf.game_name.clone()));
    }
    vec
}

fn main() {
    pretty_env_logger::init();
    std::process::exit(run::<Model>());
}