
use crate::io::{files};
use directories::{UserDirs,ProjectDirs,BaseDirs};

use std::fs::{self,File};
use std::io::{prelude::*,BufReader};
use std::collections::HashMap;
use std::path::{Path,PathBuf};
use serde::Deserialize;


struct SupportedGame {
    game_id: String,
    folder_name: PathBuf,
}

impl SupportedGame {
    pub fn new(id: &str, folder: &str) -> Self {
        SupportedGame {game_id: id.to_owned(), folder_name: PathBuf::from(folder)}
    }
}

pub struct ArgOptions {
    pub config_path: PathBuf,
    pub extract: bool,
    pub dry_run: bool,
    pub verbose: bool,
    pub game_id: String,
    pub patch_name: String,
}

impl ArgOptions {
    pub fn new(config_path: PathBuf, extract: bool, dry_run: bool, verbose: bool, game_id: String, patch_name: String) -> Self {
        ArgOptions {config_path,extract,dry_run,verbose,game_id,patch_name}
    }
    pub fn folder_name(&self) -> String {
        let mut mod_folder = self.patch_name.clone();
        mod_folder.make_ascii_lowercase();
        mod_folder
    }
}

#[derive(Deserialize,Debug)]
pub struct ConfigOptions {
    pub game_name: String,
    pub mod_path: PathBuf,
    pub data_path: PathBuf,
    pub valid_paths: Vec<PathBuf>,
    pub valid_extensions: Vec<String>,
    pub workshop_enabled: bool,
    pub new_launcher: bool,
}

#[derive(Deserialize,Debug)]
struct ConfigListItem {
    datapath: String,
    modpath: String,
    valid_paths: Vec<String>,
    valid_extensions: Vec<String>,
    workshop: bool,
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
            workshop_enabled: config_info.workshop,
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
            valid_paths: valid_paths,
            valid_extensions: config_option.valid_extensions.clone(),
            workshop: config_option.workshop_enabled,
            new_launcher: config_option.new_launcher,
        };
        (config_option.game_name.clone(),config_list_item)
    }
}

impl ConfigOptions {
    pub fn new(game_name: String, mod_path: PathBuf, data_path: PathBuf, valid_paths: &[PathBuf], valid_extensions: &[String], workshop_enabled: bool, new_launcher: bool) -> Self {
        ConfigOptions {game_name,mod_path,data_path,valid_paths: valid_paths.to_vec(), valid_extensions: valid_extensions.to_vec(),workshop_enabled,new_launcher}
    }

    pub fn new_with_str(game_name: String, mod_path: PathBuf, data_path: PathBuf, valid_paths: &[&str], valid_extensions: &[&str], workshop_enabled: bool, new_launcher: bool) -> Self {
        let valid_paths: Vec<PathBuf> = valid_paths.iter().map(|path| PathBuf::from(path)).collect();
        let valid_extensions: Vec<String> = valid_extensions.iter().map(|&extension| extension.to_owned()).collect();
        ConfigOptions::new(game_name,mod_path,data_path,&valid_paths,&valid_extensions,workshop_enabled,new_launcher)
    }

    pub fn update_paths(self,new_mod_path: PathBuf, new_data_path: PathBuf) -> Self {
        let mut new_options = self;
        new_options.mod_path = new_mod_path;
        new_options.data_path = new_data_path;
        new_options
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

pub fn parse_user_config(arguments: &ArgOptions, defaults: bool) -> Result<ConfigOptions,Box<dyn std::error::Error>> {
    let configs = if arguments.config_path.components().count() == 0 {
        fetch_user_configs(defaults)?
    } else {
        parse_configs(&arguments.config_path)?
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
            let configs = generate_default_configs();
            let _ok = store_user_configs(&configs)?;
            return Ok(configs);
        } else {
            return Err(Box::new(e));
        }
    } else {
        return match parse_configs(&config_path) {
            Ok(v) => Ok(v),
            Err(e) => Err(Box::new(e)),
        };
    }
    Ok(generate_default_configs())
}

pub fn store_user_configs(options: &[ConfigOptions]) -> Result<(),Box< dyn std::error::Error>> {
    let user_path = ProjectDirs::from("com", "Parker Okonek", "Paradox Merger").expect("Something went wrong in reading the user dirs.");
    let _e = fs::create_dir_all(user_path.config_dir())?;
    let config_path = user_path.config_dir().join("merger.toml");

    let config_file = File::open(&config_path);
    Ok(())
}

fn generate_default_configs() -> Vec<ConfigOptions> {
    let mut game_paths = HashMap::new();
    let steamapps_dir = get_default_steamapps_dir();
    for game in supported_games() {
        let game_dir = steamapps_dir.as_path().join(&game.folder_name);
        if game_dir.exists() {
            game_paths.insert(game.game_id,game_dir);
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
            false,
            false
        );
        config_options.push(ck2_config);
    }
    if let Some(path) = game_paths.get("CK3") {
        let ck3_config = ConfigOptions::new_with_str("CK3".to_owned(),
            get_user_game_data_dir(Some("Crusader Kings III"), true),
            path.join("game"),
            &["common","content_source","events","fonts","gfx","gui","history","localization","map_data","music","notifications","sound","tests"],
            &["gfx","txt","csv","gui","xml","settings","compound","editordata"],
            true,
            true
        );
        config_options.push(ck3_config);
    }
    if let Some(path) = game_paths.get("EU4") {
        
    }
    if let Some(path) = game_paths.get("HOI4") {
        
    }
    if let Some(path) = game_paths.get("Stellaris") {
        
    }
    if let Some(path) = game_paths.get("VIC2") {
        
    }
    
    config_options
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
        let home_base = BaseDirs::new().expect("Something went wrong in reading the base dirs.");;
        PathBuf::from(home_base.home_dir().join(r#"/Library/Application Support/Steam/steamapps/common"#))
    // Otherwise, assume Linux
    } else {
        let home_base = BaseDirs::new().expect("Something went wrong in reading the base dirs.");;
        PathBuf::from(home_base.home_dir().join(r#"/.steam/steam/steamapps/common"#))
    }
}

fn get_user_game_data_dir(folder_name: Option<&str>, new_launcher: bool) -> PathBuf {
    PathBuf::new()
}