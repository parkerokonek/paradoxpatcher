#![recursion_limit="512"]
mod vgtk_ext;

use vgtk::ext::*;
use vgtk::lib::gio::ApplicationFlags;
use vgtk::lib::gtk::*;
use vgtk::{gtk, run, Component, UpdateAction, VNode};
use std::path::{PathBuf,Path};

use vgtk_ext::*;
use std::env;

use paradoxmerger::configs::{ConfigOptions,fetch_user_configs,ArgOptions};
use paradoxmerger::{generate_entire_mod_list,ModPack,ModStatus,ModToken,auto_merge,extract_all_files};

const H_PADDING: i32 = 10;
const V_PADDING: i32 = 20;

trait Renderable {
    fn render(&self) ->VNode<Model>;
}

impl Renderable for ModStatus {
    fn render(&self) -> VNode<Model> {
        let token = self.special_number();
        gtk! {
        <ListBoxRow halign=Align::Start>
            <Box>
            <CheckButton active=self.status() on toggled=|_| Message::ToggleModStatus(token)/>
            <Label label=self.name() />
            </Box>
        </ListBoxRow>
        }
    }
}

#[derive(Clone, Debug)]
struct Model {
    configs: Vec<ConfigOptions>,
    mod_pack: ModPack,
    config_selected: Option<String>,
    output_path: PathBuf,
    extract_all: bool,
    scan_auto: bool,
    patch_name: String,
}

impl Default for Model {
    fn default() -> Self {
        Self {
            configs: fetch_user_configs(true).unwrap_or(Vec::new()),
            mod_pack: ModPack::default(),
            config_selected: None,
            output_path: env::current_dir().unwrap_or_default(),
            extract_all: false,
            scan_auto: false,
            patch_name: String::from("Merged Patch"),
        }
    }
}

impl Model {
    fn get_current_config(&self) -> Option<&ConfigOptions> {
        if let Some(s) = &self.config_selected {
            self.configs.iter().find(|m| m.game_name == s.clone())
        } else {
            None
        }
    }
}

#[derive(Clone, Debug)]
enum Message {
    Exit,
    ConfigSelected(Option<String>),
    ToggleScan,
    ToggleExtract,
    ManualScan,
    SetPatchName(String),
    SetOutputPath(String),
    SaveLoadOrder,
    GeneratePatch,
    ToggleModStatus(ModToken),
}

async fn print_ok_dialog() -> vgtk::lib::gtk::ResponseType {
    vgtk::message_dialog(vgtk::current_window().as_ref(), DialogFlags::MODAL, MessageType::Info, ButtonsType::OkCancel, true, "Successfully merged").await
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
                self.mod_pack = match s {
                    Some(text) => update_mod_pack(text, self.scan_auto, &self.configs),
                    None => ModPack::default(),
                };
                UpdateAction::Render
            },
            Message::ToggleScan => {
                self.scan_auto = !self.scan_auto;
                UpdateAction::None
            },
            Message::ToggleExtract => {
                self.extract_all = !self.extract_all;
                UpdateAction::None
            },
            Message::ManualScan => {
                if let Some(config) = &self.get_current_config() {
                let vanilla = paradoxmerger::files_in_vanilla(&config);
                let val_ref: Vec<&Path> = vanilla.iter().map(|x| x.as_path()).collect();
                self.mod_pack.register_vanilla(&val_ref);
            
                self.mod_pack.generate_conflicts();
                }
                UpdateAction::None
            },
            Message::SetPatchName(patch_name) => {
                self.patch_name = patch_name.clone();
                UpdateAction::None
            },
            Message::SetOutputPath(output_path) => {
                self.output_path = PathBuf::from(output_path);
                UpdateAction::None
            },
            Message::SaveLoadOrder => {
                if let Some(config) = &self.get_current_config() { 
                    let load_order = self.mod_pack.load_order();
                    let _res = paradoxmerger::set_entire_mod_list(&config.mod_path, config.new_launcher,&load_order);
                }
                UpdateAction::None
            },
            Message::GeneratePatch => {
                if let Some(conf_name) = &self.config_selected {
                    let conf = self.configs.iter().find(|c| &c.game_name == conf_name);
                    let conf = match conf {Some(c) => c, _ => return UpdateAction::None};

                    let args = ArgOptions::new(PathBuf::new(), self.extract_all, false, true, String::new(), self.patch_name.clone());

                    if self.extract_all {
                        extract_all_files(&self.mod_pack, &args, &conf, false, &self.output_path);
                    }
                    let merge_result = auto_merge(conf,&args,&self.mod_pack);
                    
                    
                    let _res2 = vgtk::run_dialog::<MergeDialog>(vgtk::current_window().as_ref());
                }
                UpdateAction::None
            },
            Message::ToggleModStatus(token) => {
                match self.mod_pack.toggle_by_token(token) {
                    Some(_) => (),
                    None => {eprintln!("Could not verify token!");},
                };
                UpdateAction::None
            }
        }
    }

    fn view(&self) -> VNode<Model> {
        gtk! {
            <Application::new_unwrap(Some("com.parkerokonek.paradoxmerger"), ApplicationFlags::empty())>
                <Window border_width=20 title="Parker's Paradox Patcher".to_owned() on destroy=|_| Message::Exit>
                <Box spacing=H_PADDING>
                <Frame
                property_width_request=350
                property_height_request=450>
                <ScrolledWindow>
                <ListBox border_width=10>
                {
                    self.mod_pack.load_order().iter().map(ModStatus::render)
                }
                </ListBox>
                </ScrolledWindow>
                </Frame>
                <Box orientation=Orientation::Vertical spacing=V_PADDING >
                <Box spacing=H_PADDING>
                    <ComboBoxText items=list_config_entries(&self.configs) selected=self.config_selected.clone() tooltip_text="Select a game to patch.".to_owned() on changed=|e| Message::ConfigSelected(to_string_option(e.get_active_text())) />
                    <Button label=" + ".to_owned() tooltip_text="Modify game entries.".to_owned() />
                </Box>
                <Box spacing=H_PADDING>
                    <Label label="Output Directory:".to_owned() />
                    <Entry property_width_request=300 text=self.output_path.to_string_lossy().as_ref().clone() on changed=|a| Message::SetOutputPath(gstring_to_string(a.get_text())) />
                </Box>
                <Box spacing=H_PADDING>
                    <Label label="Extract all files: "/>
                    <CheckButton on toggled=|_| Message::ToggleExtract />
                    <Label label="Scan Automatically (SLOW): "/>
                    <CheckButton on toggled=|_| Message::ToggleScan />
                </Box>
                <Box spacing=H_PADDING>
                    <Label label="Output Mod Title:"/>
                    <Entry property_width_request=200 text=self.patch_name.clone() on changed=|a| Message::SetPatchName(gstring_to_string(a.get_text()))/>
                </Box>
                <Box spacing=H_PADDING>
                    <Button label="Save Load Order".to_owned() on clicked=|_| Message::SaveLoadOrder />
                    <Button label="Scan Mod Conflicts".to_owned() on clicked=|_| Message::ManualScan />
                    <Button label="Generate Patch".to_owned() on clicked=|_| Message::GeneratePatch />
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

fn update_mod_pack(selected_idx: String, register_conflicts: bool, configs: &[ConfigOptions]) -> ModPack {
    let conf: Option<&ConfigOptions> = configs.iter().find(|m| m.game_name == selected_idx);
    if let Some(config) = conf {
        let mod_list = generate_entire_mod_list(&config.mod_path, config.new_launcher);
        let mut new_pack = ModPack::default()
            .restrict_paths(&config.valid_paths)
            .restrict_extensions(&config.valid_extensions);

        if register_conflicts {
            let vanilla = paradoxmerger::files_in_vanilla(&config);
            let val_ref: Vec<&Path> = vanilla.iter().map(|x| x.as_path()).collect();
            new_pack.register_vanilla(&val_ref);
            
            new_pack.add_mods(&mod_list, true, true);
        } else {
            new_pack.add_mods(&mod_list, false, false);
        }
        
        new_pack
    } else {
        ModPack::default()
    }
}

// Our pop up window to indicate merging has finished.MergeDialog
// Later this will have a progress bar indicating how much of the files have been merged
pub struct MergeDialog;
/*
impl MergeDialog {
    async fn run() -> i32 {
        vgtk::run_dialog::<MergeDialog>(vgtk::current_window().as_ref())
    }
}*/

impl Default for MergeDialog {
    fn default() -> Self {
        MergeDialog {}
    }
}

impl Component for MergeDialog {
    type Message = ();
    type Properties = ();

    fn view(&self) -> VNode<Self> {
        gtk! {
            <Dialog::with_buttons(
                Some("Merging Mods"),
                None as Option<&Window>,
                DialogFlags::MODAL,
                &[("Ok", ResponseType::Ok)]
            )>
            </Dialog>
        }
    }
}


fn main() {
    pretty_env_logger::init();
    std::process::exit(run::<Model>());
}