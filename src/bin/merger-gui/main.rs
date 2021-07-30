#![recursion_limit = "512"]
mod vgtk_ext;

use vgtk::ext::*;
use vgtk::lib::gio::ApplicationFlags;
use vgtk::lib::gtk::*;
use vgtk::{gtk, run, Component, UpdateAction, VNode};

use std::path::Path;

use vgtk_ext::*;

use paradoxmerger::configs::{ConfigOptions, MergerSettings};
use paradoxmerger::{ModMerger, ModPack, ModStatus, ModToken};

const H_PADDING: i32 = 10;
const V_PADDING: i32 = 20;

trait Renderable {
    fn render(&self) -> VNode<Model>;
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
    gui_settings: MergerSettings,
    config_selected: Option<String>,
    scan_auto: bool,
}

impl Default for Model {
    fn default() -> Self {
        let settings = MergerSettings::default();

        Self {
            configs: ConfigOptions::fetch_user_configs(true).unwrap_or(Vec::new()),
            mod_pack: ModPack::default(),
            gui_settings: settings,
            config_selected: None,
            scan_auto: false,
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

impl Component for Model {
    type Message = Message;
    type Properties = ();

    fn update(&mut self, msg: Self::Message) -> UpdateAction<Self> {
        match msg {
            Message::Exit => {
                vgtk::quit();
                UpdateAction::None
            }
            Message::ConfigSelected(s) => {
                self.config_selected = s.clone();
                self.mod_pack = match s {
                    //TODO: Clean up
                    Some(text) => {
                        update_mod_pack(text, self.scan_auto, &self.configs, &self.gui_settings)
                    }
                    None => ModPack::default(),
                };
                UpdateAction::Render
            }
            Message::ToggleScan => {
                self.scan_auto = !self.scan_auto;
                UpdateAction::None
            }
            Message::ToggleExtract => {
                self.gui_settings.extract_toggle();
                UpdateAction::None
            }
            Message::ManualScan => {
                if let Some(config) = &self.get_current_config() {
                    let vanilla = ModMerger::files_in_vanilla(&config);
                    let val_ref: Vec<&Path> = vanilla.iter().map(|x| x.as_path()).collect();
                    self.mod_pack.register_vanilla(&val_ref);

                    self.mod_pack.generate_conflicts();
                }
                UpdateAction::None
            }
            Message::SetPatchName(patch_name) => {
                self.gui_settings.patch_name = patch_name.clone();
                UpdateAction::None
            }
            Message::SetOutputPath(output_path) => {
                self.gui_settings.patch_path = output_path;
                UpdateAction::None
            }
            Message::SaveLoadOrder => {
                if let Some(config) = &self.get_current_config() {
                    let load_order = self.mod_pack.load_order();
                    let _res = ModMerger::set_entire_mod_list(
                        &config.mod_path,
                        config.new_launcher,
                        &load_order,
                    );
                }
                UpdateAction::None
            }
            Message::GeneratePatch => {
                if let Some(conf_name) = &self.config_selected {
                    let conf = self.configs.iter().find(|c| &c.game_name == conf_name);
                    let conf = match conf {
                        Some(c) => c,
                        _ => return UpdateAction::None,
                    };

                    let mut mod_merger = ModMerger::new(
                        self.gui_settings.extract,
                        &self.gui_settings.patch_name,
                        Path::new(&self.gui_settings.patch_path),
                    );
                    mod_merger.set_config(conf.clone());
                    //TODO: Print Errors
                    let merge_result = mod_merger.merge_and_save(&self.mod_pack);
                    if let Err(merge_err) = merge_result {
                        eprintln!("{}", merge_err);
                    }

                    MergeDialog::run();
                }
                UpdateAction::None
            }
            Message::ToggleModStatus(token) => {
                match self.mod_pack.toggle_by_token(token) {
                    Some(_) => (),
                    None => {
                        eprintln!("Could not verify token!");
                    }
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
                    <Entry property_width_request=300 text=self.gui_settings.patch_path.clone() on changed=|a| Message::SetOutputPath(gstring_to_string(a.get_text())) />
                </Box>
                <Box spacing=H_PADDING>
                    <Label label="Extract all files: "/>
                    <CheckButton on toggled=|_| Message::ToggleExtract />
                    <Label label="Scan Automatically (SLOW): "/>
                    <CheckButton on toggled=|_| Message::ToggleScan />
                </Box>
                <Box spacing=H_PADDING>
                    <Label label="Output Mod Title:"/>
                    <Entry property_width_request=200 text=self.gui_settings.patch_name.clone() on changed=|a| Message::SetPatchName(gstring_to_string(a.get_text()))/>
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

fn list_config_entries(configs: &[ConfigOptions]) -> Vec<(Option<String>, String)> {
    let mut vec = Vec::new();
    for conf in configs {
        vec.push((Some(conf.game_name.clone()), conf.game_name.clone()));
    }
    vec
}

fn update_mod_pack(
    selected_idx: String,
    register_conflicts: bool,
    configs: &[ConfigOptions],
    gui_settings: &MergerSettings,
) -> ModPack {
    let conf: Option<&ConfigOptions> = configs.iter().find(|m| m.game_name == selected_idx);
    if let Some(config) = conf {
        let mod_merger = ModMerger::new(
            gui_settings.extract,
            &gui_settings.patch_name,
            Path::new(&gui_settings.patch_path),
        );
        //mod_merger.set_config(*config);

        mod_merger
            .using_config(config.clone())
            .mod_pack_from_enabled(register_conflicts)
            .unwrap_or_else(|_| ModPack::default())
    } else {
        ModPack::default()
    } /*
      match conf {
          Some(config) if let Ok(m) = ModPack::default()
                                      .with_config(*config)
                                      .mod_pack_from_enabled(register_conflicts)
                                       => {m},
          _ => ModPack::default(),
      }*/
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

impl MergeDialog {
    fn run() {
        let _future = vgtk::run_dialog::<MergeDialog>(vgtk::current_window().as_ref());
    }
}

fn main() {
    pretty_env_logger::init();
    std::process::exit(run::<Model>());
}
