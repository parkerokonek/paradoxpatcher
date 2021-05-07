
use crate::io::{files,re};
use directories::{ProjectDirs,BaseDirs};

use std::fs::{self,File};
use std::io::{prelude::*};
use std::collections::HashMap;
use std::path::{Path,PathBuf};
use serde::{Deserialize,Serialize};

use lazy_static::lazy_static;
use regex::Regex;

lazy_static! {
    static ref RE_VDF_PATH: Regex = if cfg!(windows) {
        Regex::new(r#""[^"]*\\[^"]*"\s*"#).expect("Error in compiling Windows VDF Path Regex")
    } else {
        Regex::new(r#""[^"]*/[^"]*"\s*"#).expect("Error in compiling Unix VDF Path Regex")
    };
}

struct SupportedGame {
    game_id: String,
    folder_name: PathBuf,
}

impl SupportedGame {
    pub fn new(id: &str, folder: &str) -> Self {
        SupportedGame {game_id: id.to_owned(), folder_name: PathBuf::from(folder)}
    }
}

#[derive(Deserialize,Serialize,Debug,Clone)]
pub struct MergerSettings {
    pub config_path: PathBuf,
    pub extract: bool,
    pub dry_run: bool,
    pub verbose: bool,
    pub game_id: String,
    pub patch_name: String,
    pub patch_path: String,
}

impl Default for MergerSettings {
    fn default() -> Self {
        let user_path = ProjectDirs::from("com", "Parker Okonek", "Paradox Merger").expect("Something went wrong in reading the user dirs.");
        //TODO: i dunno
        //let _e = fs::create_dir_all(user_path.config_dir())?;
        let config_path = user_path.config_dir().join("merger.toml");
        let patch_name = String::from("Merged Patch");
        let patch_path = String::from(files::relative_folder_path(Path::new(""),Path::new("")).unwrap_or_default().to_string_lossy());
        let game_id = String::new();
        let extract = false;
        let verbose = false;
        let dry_run = false;
        
        MergerSettings {config_path, extract, dry_run, verbose, game_id, patch_name, patch_path}
    }
}

impl MergerSettings {
    pub fn new(config_path: PathBuf, extract: bool, dry_run: bool, verbose: bool, game_id: String, patch_name: String, patch_path: String) -> Self {
        MergerSettings {config_path,extract,dry_run,verbose,game_id,patch_name,patch_path}
    }

    pub fn folder_name(&self) -> String {
        let mut mod_folder = self.patch_name.clone();
        mod_folder.make_ascii_lowercase();
        mod_folder
    }

    // TODO: Maybe do some custom error types here
    pub fn fetch_from_file(path_to_settings: &Path) -> Result<MergerSettings, Box<dyn std::error::Error>> {
        let mut settings_file = File::open(path_to_settings)?;
        let mut contents = String::new();
        settings_file.read_to_string(&mut contents)?;
        let merger_settings: MergerSettings = match toml::from_str(&contents) {
            Ok(sett) => sett,
            _ => Default::default(),
        };
        
        Ok(merger_settings)
    }

    pub fn store_in_file(&self, path_to_settings: &Path) -> Result<(), Box<dyn std::error::Error>> {
        let mut settings_file = File::create(path_to_settings)?;
        // TODO: Okay yeah this does need custom errors
        let contents = toml::to_string(&self)?;

        settings_file.write_all(contents.as_bytes()).map_err(Box::from)
    }

    pub fn extract_toggle(&mut self) {
        self.extract = !self.extract;
    }
}

#[derive(Deserialize,Debug,Clone)]
pub struct ConfigOptions {
    pub game_name: String,
    pub mod_path: PathBuf,
    pub data_path: PathBuf,
    pub valid_paths: Vec<PathBuf>,
    pub valid_extensions: Vec<String>,
    pub no_transcode: Vec<String>,
    pub new_launcher: bool,
}

#[derive(Deserialize,Serialize,Debug)]
struct ConfigListItem {
    datapath: String,
    modpath: String,
    valid_paths: Vec<String>,
    valid_extensions: Vec<String>,
    no_transcode: Vec<String>,
    new_launcher: bool,
}

type TomlConfigItem = (String,ConfigListItem);

impl From<TomlConfigItem> for ConfigOptions {
    fn from((game_id, config_info): TomlConfigItem) -> Self {
        let valid_paths: Vec<PathBuf> = config_info.valid_paths.iter().map(|x| PathBuf::from(&x)).collect();
        let valid_extensions: Vec<String> = config_info.valid_extensions;
        ConfigOptions {
            game_name: game_id,
            mod_path: PathBuf::from(config_info.modpath),
            data_path: PathBuf::from(config_info.datapath),
            valid_paths,
            valid_extensions,
            no_transcode: config_info.no_transcode,
            new_launcher: config_info.new_launcher,
        }
    }
}

impl From<&ConfigOptions> for TomlConfigItem {
    fn from(config_option: &ConfigOptions) -> Self {
        let valid_paths: Vec<String> = config_option.valid_paths.iter().map(|x| x.to_string_lossy().to_string()).collect();
        let config_list_item = ConfigListItem {
            datapath: config_option.data_path.to_string_lossy().to_string(),
            modpath: config_option.mod_path.to_string_lossy().to_string(),
            valid_paths,
            valid_extensions: config_option.valid_extensions.clone(),
            no_transcode: config_option.no_transcode.clone(),
            new_launcher: config_option.new_launcher,
        };
        (config_option.game_name.clone(),config_list_item)
    }
}

impl ConfigOptions {
    pub fn new(game_name: String, mod_path: PathBuf, data_path: PathBuf, valid_paths: &[PathBuf], valid_extensions: &[String], no_transcode: &[String], new_launcher: bool) -> Self {
        ConfigOptions {game_name,mod_path,data_path,valid_paths: valid_paths.to_vec(), valid_extensions: valid_extensions.to_vec(),no_transcode: no_transcode.to_vec(),new_launcher}
    }

    pub fn new_with_str(game_name: String, mod_path: PathBuf, data_path: PathBuf, valid_paths: &[&str], valid_extensions: &[&str], no_transcode: &[&str], new_launcher: bool) -> Self {
        let valid_paths: Vec<PathBuf> = valid_paths.iter().map(PathBuf::from).collect();
        let valid_extensions: Vec<String> = valid_extensions.iter().map(|&extension| extension.to_owned()).collect();
        let no_transcode: Vec<String> = no_transcode.iter().map(|&code| code.to_owned()).collect();
        ConfigOptions::new(game_name,mod_path,data_path,&valid_paths,&valid_extensions,&no_transcode,new_launcher)
    }

    pub fn update_paths(self,new_mod_path: PathBuf, new_data_path: PathBuf) -> Self {
        let mut new_options = self;
        new_options.mod_path = new_mod_path;
        new_options.data_path = new_data_path;
        new_options
    }



pub fn parse_user_config(arguments: &MergerSettings, defaults: bool) -> Result<ConfigOptions,Box<dyn std::error::Error>> {
    let configs = if arguments.config_path.components().count() == 0 {
        //TODO: make this depend on the actual object
        ConfigOptions::fetch_user_configs(defaults)?
    } else {
        ConfigOptions::parse_configs(&arguments.config_path)?
    };
    if arguments.game_id.is_empty() {
        match configs.into_iter().next() {
            None => Err(Box::new(std::io::Error::new(std::io::ErrorKind::Other, "No games found in configuration file."))),
            Some(value) => Ok(value),
        }
    } else {
        match configs.into_iter().find(|x| x.game_name == arguments.game_id) {
            Some(conf) => Ok(conf),
            None => Err(Box::new(std::io::Error::new(std::io::ErrorKind::Other, "Game not found in configuration file."))),
        }
    }
}

fn parse_configs(config_path: &Path) -> Result<Vec<ConfigOptions>,std::io::Error> {
    let config_file = File::open(config_path);
    if let Ok(mut file_ok) = config_file {
        let mut contents = String::new();
        let err = file_ok.read_to_string(&mut contents);
        if let Ok(_e) = err {
            let configs_untyped: toml::value::Table = toml::from_str(&contents).expect("Configuration file could not be read!");
            let mut game_configs = Vec::new();

            for (game_id,config_untyped) in configs_untyped {
                let config_string = config_untyped.to_string();
                let config: ConfigListItem = toml::from_str(&config_string).expect("Malformed configuration for game found."); 
                game_configs.push(ConfigOptions::from((game_id,config)));
            }
            
            return Ok(game_configs);
        } else if let Err(e) = err {
            return Err(e);
        }
    } else if let Err(file_bad) = config_file {
        return Err(file_bad);
    }
    Err(std::io::Error::new(std::io::ErrorKind::Other, "This is a config parsing error that should never appear."))
}

pub fn fetch_user_configs(defaults: bool) -> Result<Vec<ConfigOptions>, Box<dyn std::error::Error>> {
    let user_path = ProjectDirs::from("com", "Parker Okonek", "Paradox Merger").expect("Something went wrong in reading the user dirs.");
    let _e = fs::create_dir_all(user_path.config_dir())?;
    let config_path = user_path.config_dir().join("merger.toml");

    let config_file = File::open(&config_path);
    if let Err(e) = config_file {
        if defaults {
            println!("Generating new default configs");
            //TODO: Implement default or something
            let configs = ConfigOptions::generate_default_configs();
            let _ok = ConfigOptions::store_user_configs(&configs)?;
            Ok(configs)
        } else {
            Err(Box::new(e))
        }
    } else {
        match ConfigOptions::parse_configs(&config_path) {
            Ok(v) => Ok(v),
            Err(e) => Err(Box::new(e)),
        }
    }
}

pub fn store_user_configs(options: &[ConfigOptions]) -> Result<(),Box< dyn std::error::Error>> {
    let user_path = ProjectDirs::from("com", "Parker Okonek", "Paradox Merger").expect("Something went wrong in reading the user dirs.");
    let _e = fs::create_dir_all(user_path.config_dir())?;
    let config_path = user_path.config_dir().join("merger.toml");

    let mut config_file = File::create(&config_path)?;
    let mut config_contents = String::new();
    for (entry_name,config_item) in options.iter().map(TomlConfigItem::from) {
        let toml_data = toml::to_string(&config_item)?;
        config_contents.push('[');
        config_contents.push_str(&entry_name);
        config_contents.push_str("]\n");
        config_contents.push_str(&toml_data);
        config_contents.push('\n');
    }

    config_file.write_all(config_contents.as_bytes())?;
    Ok(())
}

fn generate_default_configs() -> Vec<ConfigOptions> {
    let mut game_paths = HashMap::new();
    let steamapps_dirs = get_all_steam_library_folders();
    for game in supported_games() {
        for steam_dir in &steamapps_dirs {
            let game_dir = steam_dir.as_path().join(&game.folder_name);
            if game_dir.exists() {
                game_paths.insert(game.game_id,game_dir);
                break;
            }
        }
    }

    let mut config_options = Vec::new();
    if let Some(path) = game_paths.get("CK2") {
        let ck2_config = ConfigOptions::new_with_str(
            "CK2".to_owned(),
            get_user_game_data_dir(Some("Crusader Kings II"), false),
            path.clone(),
            &["history", "common", "decisions", "events", "localisation", "gfx", "interface", "music", "soundtrack", "tutorial"],
            &["gfx","txt","csv","gui","xml"],
            &[],
            false
        );
        config_options.push(ck2_config);
    }
    if let Some(path) = game_paths.get("CK3") {
        let ck3_config = ConfigOptions::new_with_str("CK3".to_owned(),
            get_user_game_data_dir(Some("Crusader Kings III"), true),
            path.join("game"),
            &["common","content_source","events","fonts","gfx","gui","history","localization","map_data","music","notifications","sound","tests"],
            &["gfx","txt","csv","gui","xml","settings","compound","editordata","yml"],
            &["yml"],
            true
        );
        config_options.push(ck3_config);
    }
    if let Some(path) = game_paths.get("EU4") {
        let eu4_config = ConfigOptions::new_with_str(
            "EU4".to_owned(),
            get_user_game_data_dir(Some("Europa Universalis IV"), true),
            path.clone(),
            &["common", "customizable_localization", "decisions", "events", "gfx", "hints", "history", "interface", "localisation", "map", "missions", "music", "sound", "soundtrack", "tests", "tutorial"],
            &["gfx","txt","csv","gui","xml","yml"],
            &["yml"],
            true
        );
        config_options.push(eu4_config);
    }
    if let Some(path) = game_paths.get("HOI4") {
        let hoi4_config = ConfigOptions::new_with_str(
            "HOI4".to_owned(),
            get_user_game_data_dir(Some("Hearts of Iron IV"), true),
            path.clone(),
            &["common", "documentation", "events", "gfx", "history", "interface", "localisation", "map", "music", "portraits", "script", "sound", "tests", "tutorial", "wiki"],
            &["gfx","txt","csv","gui","xml","yml"],
            &["yml"],
            true
        );
        config_options.push(hoi4_config);
    }
    if let Some(path) = game_paths.get("Stellaris") {
        let stellaris_config = ConfigOptions::new_with_str(
            "Stellaris".to_owned(),
            get_user_game_data_dir(Some("Stellaris"), true),
            path.clone(),
            &["common", "events", "flags", "fonts", "gfx", "interface", "locales", "localisation", "map", "music", "prescripted_countries", "sound"],
            &["gfx","txt","csv","gui","xml","yml"],
            &["yml"],
            true
        );
        config_options.push(stellaris_config);
        
    }
    if let Some(_path) = game_paths.get("VIC2") {
        //TODO: Implement this game default config
        eprintln!("Game not yet implemented! VIC2");
        
    }
    
    config_options
}
}

    // Don't worry about it
    fn supported_games() -> Vec<SupportedGame> {
        vec![
            SupportedGame::new("CK2", "Crusader Kings II"),
            SupportedGame::new("CK3","Crusader Kings III"),
            SupportedGame::new("EU4","Europa Universalis IV"),
            SupportedGame::new("HOI4", "Hearts of Iron IV"),
            SupportedGame::new("Stellaris","Stellaris"),
            SupportedGame::new("VIC2", "Victoria 2"),
        ]
    }


fn get_default_steamapps_dir() -> PathBuf {
    //Check if windows (x86 or 64)
    if cfg!(windows) {
        if cfg!(x86) {
            PathBuf::from(r#"C:\Program Files\Steam\steamapps\common"#)
        } else {
            PathBuf::from(r#"C:\Program Files (x86)\Steam\steamapps\common"#)
        }
    } else if cfg!(macos) {
        let home_base = BaseDirs::new().expect("Something went wrong in reading the base dirs.");
        home_base.home_dir().join(r#"Library/Application Support/Steam/steamapps/common"#)
    // Otherwise, assume Linux
    } else {
        let home_base = BaseDirs::new().expect("Something went wrong in reading the base dirs.");
        home_base.home_dir().join(r#".steam/steam/steamapps/common"#)
    }
}

fn get_all_steam_library_folders() -> Vec<PathBuf> { 
    let mut library_folders = vec![get_default_steamapps_dir()];
    let steamapps_dir = library_folders[0].parent().expect("Something went horribly wrong with getting the default steamapps directory.");

    let extra_folders = files::fgrep(&steamapps_dir.join("libraryfolders.vdf"), &RE_VDF_PATH, true);

    for extra_folder in extra_folders.iter().map(|s| re::trim_quotes(&s)) {
        println!("{}",extra_folder);
        library_folders.push(PathBuf::from(extra_folder).join("steamapps/common"));
    }
    
    library_folders
}

fn get_user_game_data_dir(folder_name: Option<&str>, new_launcher: bool) -> PathBuf {
    let home_base = BaseDirs::new().expect("Something went wrong in reading the base dirs.");
    let base = if cfg!(windows) || cfg!(macos) {
        home_base.home_dir().join("Documents/Paradox Interactive")
    //Otherwise, assume Linux
    } else if new_launcher {
        home_base.home_dir().join(".local/share/Paradox Interactive")
    } else {
        home_base.home_dir().join(".paradoxinteractive")
    };

    match folder_name {
        Some(folder) => base.join(folder),
        None => base,
    }
}