#![recursion_limit="512"]
mod vgtk_ext;

use vgtk::ext::*;
use vgtk::lib::gio::ApplicationFlags;
use vgtk::lib::gtk::*;
use vgtk::{gtk, run, Component, UpdateAction, VNode};
use std::path::PathBuf;

use vgtk_ext::*;
use std::env;

use paradoxmerger::configs::{ConfigOptions,fetch_user_configs};
use paradoxmerger::{ModInfo,generate_enabled_mod_list,generate_entire_mod_list};

const H_PADDING: i32 = 10;
const V_PADDING: i32 = 20;

#[derive(Clone, Debug)]
struct ModEntry {
    enabled: bool,
    mod_info: ModInfo,
}

impl ModEntry {
    fn enabled(&self) -> bool {
        self.enabled
    }

    fn mod_info(&self) -> &ModInfo {
        &self.mod_info
    }

    fn get_name(&self) -> &str {
        let mod_info = self.mod_info();
        mod_info.get_name()
    }

    fn vec_from_tuple(source: Vec<(bool,ModInfo)>) -> Vec<ModEntry> {
        let mut mods = Vec::new();
        for entry in source {
            mods.push(ModEntry {enabled: entry.0, mod_info: entry.1});
        }
        mods
    }

    fn render(&self) -> VNode<Model> {
        gtk! {
        <ListBoxRow halign=Align::Start>
            <Box>
            <CheckButton active=self.enabled() />
            <Label label=self.get_name().to_owned() />
            </Box>
        </ListBoxRow>
        }
    }
}

#[derive(Clone, Debug)]
struct Model {
    configs: Vec<ConfigOptions>,
    mod_list: Vec<ModEntry>,
    config_selected: Option<String>,
    output_path: PathBuf,
    extact_all: bool,
    scan_auto: bool,
    patch_name: String,
}

impl Default for Model {
    fn default() -> Self {
        Self {
            configs: fetch_user_configs(true).unwrap_or(Vec::new()),
            mod_list: Vec::new(),
            config_selected: None,
            output_path: env::current_dir().unwrap_or_default(),
            extact_all: false,
            scan_auto: false,
            patch_name: String::from("Merged Patch"),
        }
    }
}

#[derive(Clone, Debug)]
enum Message {
    Exit,
    ConfigSelected(Option<String>)
}

impl Component for Model {
    type Message = Message;
    type Properties = ();

    fn update(&mut self, msg: Self::Message) -> UpdateAction<Self> {
        match msg {
            Message::Exit => {
                vgtk::quit();
                UpdateAction::None
            },
            Message::ConfigSelected(s) => {
                self.config_selected = s.clone();
                if let Some(val) = s {
                    let conf: Option<&ConfigOptions> = self.configs.iter().find(|m| m.game_name == val);
                    self.mod_list = match conf {
                        None => Vec::new(),
                        Some(mod_conf) => { ModEntry::vec_from_tuple( generate_entire_mod_list( &mod_conf.mod_path, mod_conf.new_launcher )) },
                    };
                }
                UpdateAction::Render
            }
        }
    }

    fn view(&self) -> VNode<Model> {
        gtk! {
            <Application::new_unwrap(Some("com.example.paradoxmerger"), ApplicationFlags::empty())>
                <Window border_width=20 title="Parker's Paradox Patcher".to_owned() on destroy=|_| Message::Exit>
                <Box spacing=H_PADDING>
                <Frame
                property_width_request=350
                property_height_request=450>
                <ScrolledWindow>
                <ListBox border_width=10>
                {
                    self.mod_list.iter().map(ModEntry::render)
                }
                </ListBox>
                </ScrolledWindow>
                </Frame>
                <Box orientation=Orientation::Vertical spacing=V_PADDING >
                <Box spacing=H_PADDING>
                    <ComboBoxText items=list_config_entries(&self.configs) selected=self.config_selected.clone() tooltip_text="Select a game to patch.".to_owned() on changed=|e| Message::ConfigSelected(to_string_option(e.get_active_text())) />
                    <Button label="+".to_owned() tooltip_text="Modify game entries.".to_owned() />
                </Box>
                <Box spacing=H_PADDING>
                    <Label label="Output Directory:".to_owned() />
                    <Entry property_width_request=300 text=self.output_path.to_string_lossy().as_ref().clone()/>
                </Box>
                <Box spacing=H_PADDING>
                    <Label label="Extract all files: "/>
                    <CheckButton/>
                    <Label label="Scan Automatically (SLOW): "/>
                    <CheckButton/>
                </Box>
                <Box spacing=H_PADDING>
                    <Label label="Output Mod Title:"/>
                    <Entry property_width_request=200 text=self.patch_name.clone()/>
                </Box>
                <Box spacing=H_PADDING>
                    <Button label="Save Load Order".to_owned()/>
                    <Button label="Scan Mod Conflicts".to_owned()/>
                    <Button label="Generate Patch".to_owned()/>
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
        vec.push((Some(conf.game_name.clone()),conf.game_name.clone()));
    }
    vec
}

fn main() {
    pretty_env_logger::init();
    std::process::exit(run::<Model>());
}