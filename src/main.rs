mod mod_information;
use mod_information::{ModInfo,ModConflict};
use clap::{Arg,App};
use std::path::{PathBuf,Path};
use std::fs::File;
use std::io::prelude::*;
use std::io::BufReader;
use serde::Deserialize;
use regex::Regex;
use zip::read::ZipArchive;

struct ArgOptions {
    config_path: PathBuf,
    extract: bool,
    dry_run: bool,
    verbose: bool,
    game_id: String,
    patch_name: String,
}

#[derive(Deserialize,Debug)]
struct ConfigOptions {
    game_name: String,
    mod_path: PathBuf,
    data_path: PathBuf,
    valid_paths: Vec<String>,
}

#[derive(Deserialize,Debug)]
struct ConfigListItem {
    datapath: String,
    modpath: String,
    valid_paths: Vec<String>,
}

impl From<(String,ConfigListItem)> for ConfigOptions {
    fn from(from_tuple: (String, ConfigListItem)) -> Self {
        ConfigOptions {game_name: from_tuple.0, mod_path: PathBuf::from(from_tuple.1.modpath), data_path: PathBuf::from(from_tuple.1.datapath), valid_paths: from_tuple.1.valid_paths}
    }
}

fn main() {
    let args = parse_args();
    let config = parse_configs(&args).expect("Couldn't Parse the config file.");

    let mod_list: Vec<mod_information::ModInfo> = generate_mod_list(&config.mod_path);
}

fn parse_args() -> ArgOptions {
    let args = App::new("Parker's Paradox Patcher")
        .version("0.2")
        .about("Merges some mods together automatically sometimes.")
        .author("Parker Okonek")
        .arg(Arg::with_name("config")
            .short("c")
            .long("config")
            .value_name("CONFIG_FILE")
            .help("configuration file to load, defaults to current directory")
            .takes_value(true))
        .arg(Arg::with_name("extract")
            .short("x")
            .long("extract")
            .help("extract all non-conflicting files to a folder"))
        .arg(Arg::with_name("dry-run")
            .short("d")
            .long("dry-run")
            .help("list file conflicts without merging"))
        .arg(Arg::with_name("verbose")
            .short("v")
            .long("verbose")
            .help("print information about processed mods"))
        .arg(Arg::with_name("game_id")
            .required(false)
            .index(2)
            .help("game in the config to use"))
        .arg(Arg::with_name("patch_name")
            .index(1)
            .required(true)
            .help("name of the generated mod"))
        .get_matches();


    let mut config_path = PathBuf::new();
    config_path.push(args.value_of("config").unwrap_or("./merger.toml"));
    let extract = args.is_present("extract");
    let dry_run = args.is_present("dry-run");
    let verbose = args.is_present("verbose");
    let game_id = String::from(args.value_of("game_id").unwrap_or(""));
    let patch_name = String::from("merged_patch");
    
    ArgOptions{config_path,extract,dry_run,verbose,game_id,patch_name}
}

fn parse_configs(arguments: &ArgOptions) -> Result<ConfigOptions,std::io::Error> {
    let mut config_file = File::open(&arguments.config_path);
    if let Ok(mut file_ok) = config_file {
        let mut contents = String::new();
        let err = file_ok.read_to_string(&mut contents);
        if let Ok(_e) = err {
            let configs_untyped: toml::value::Table = toml::from_str(&contents).expect("Configuration file could not be read!");
            
            if !arguments.game_id.is_empty() && configs_untyped.keys().find(|k| *k == &arguments.game_id).is_none() {
                eprintln!("Game not found in configuration file.");
                return Err(std::io::Error::new(std::io::ErrorKind::Other, "Game not found in configuration file."));
            }

            let game_id: String = if !arguments.game_id.is_empty() {
                    arguments.game_id.to_string()
                }
                else {
                    configs_untyped.keys().next().expect("Bye").to_string()
            };

            let configs_untyped = configs_untyped[&game_id].to_string();
            
            let config: ConfigListItem = toml::from_str(&configs_untyped).expect("Malformed configuration for game found.");

            return Ok(ConfigOptions::from((game_id,config)));
        } else if let Err(e) = err {
            return Err(e);
        }
    } else if let Err(file_bad) = config_file {
        return Err(file_bad);
    }
    Err(std::io::Error::new(std::io::ErrorKind::Other, "This is a config parsing error that should never appear."))
}

fn file_to_string(file_path: &Path) -> Option<String> {
    let mut file = File::open(file_path);
    if let Ok(mut file_open) = file {
        let mut contents = String::new();
        if let Ok(_e) = file_open.read_to_string(&mut contents) {
            return Some(contents);
        }
    }
    
    None
}

fn fgrep(file_path: &Path, re: &Regex, all_matches: bool) -> Vec<String> {
    if let Some(input) = file_to_string(file_path) {
        return grep(&input,re,all_matches);
    }
    eprintln!("Failed to open file.\t{}",file_path.display());

    Vec::new()
}

fn grep(input: &str, re: &Regex, all_matches: bool) -> Vec<String> {
    let mut matches = re.find_iter(&input);
    if all_matches {
        return matches.map(|x| x.as_str().to_string()).collect();
    } else if let Some(valid) = matches.next() {
        println!("{}",valid.as_str());
        return vec![valid.as_str().to_string()];
    }
    
    Vec::new()
}

fn collect_dependencies(mod_path: &Path) -> Vec<String> {
    let escape = r#"\""#;
    let re_dep = Regex::new("dependencies").unwrap();

    Vec::new()
}

fn trim_quotes(input: &str) -> String {
    let left: Vec<&str> = input.split('\"').collect();
    if left.len() == 3 {
        return left[1].to_string();
    }
    String::new()
}

fn find_even_with_case(path: &Path) -> Option<PathBuf> {
    if let Ok(_file) = File::open(path) {
        return Some(PathBuf::from(&path));
    } else if path.has_root() {
        if let Ok(dir) = std::fs::read_dir(path.parent().unwrap()) {
        for entry in dir {
            if entry.is_ok() {
                let entry_path = entry.unwrap().path();
                let mut lowerpath: String = entry_path.to_str().unwrap().to_string();
                lowerpath.make_ascii_lowercase();
                let mut lowercompare: String = String::from(path.to_str().unwrap());
                lowercompare.make_ascii_lowercase();
                if lowercompare == lowerpath {
                    return Some(entry_path);
                }
            }
        }
    }
    }
    
    None
}

fn generate_single_mod(mod_path: &Path, mod_file: &Path) -> Option<mod_information::ModInfo> {
    let re_archive = Regex::new(r#"archive\s*=\s*"[^"]*\.zip""#).unwrap();
    let re_paths = Regex::new(r#"^\s*path\s*=\s*"[^"]*""#).unwrap();
    let re_names = Regex::new(r#"name\s*=\s*"[^"]*""#).unwrap();
    let re_replace = Regex::new(r#"replace_path\s*=\s*"[^"]""#).unwrap();
    let modmod_path: PathBuf = [mod_path,mod_file].iter().collect();
    let dependencies = collect_dependencies(&modmod_path);

    let modmod_content = file_to_string(&modmod_path).unwrap_or_default();

    let archive: Vec<String> = grep(&modmod_content, &re_archive, false).iter().map(|x| trim_quotes(x) ).collect();
    let path: Vec<String> = grep(&modmod_content, &re_paths, false).iter().map(|x| trim_quotes(x)).collect();
    let name: Vec<String> = grep(&modmod_content, &re_names, false).iter().map(|x| trim_quotes(x)).collect();
    let replace_paths: Vec<PathBuf> = grep(&modmod_content, &re_replace, true).iter().map(|x| PathBuf::from(trim_quotes(x))).collect();
    
    if archive.is_empty() && path.is_empty() || !archive.is_empty() && !path.is_empty() {
        eprintln!("{}\n Spaghetti-Os", &modmod_path.display());
        eprintln!("{},{}",archive.is_empty(),path.is_empty());
        return None;
    } else if name.len() == 1 && archive.len() == 1 {
        let zip_path: PathBuf = [mod_path.to_str()?,&archive[0]].iter().collect();
        println!("= {}",zip_path.display());
        let zip_path = find_even_with_case(&zip_path)?;
        let file = File::open(&zip_path).unwrap();
        let mut reader = BufReader::new(file);
        let mut zipfile = ZipArchive::new(reader).unwrap();

        let files: Vec<&str> = zipfile.file_names().collect();

        return Some(ModInfo::new(mod_path.to_path_buf(),&files,zip_path,name[0].clone(),&dependencies,&replace_paths));
    } else if name.len() == 1 && path.len() == 1 {
        eprintln!("HMMM");
        return None;
    }

    None
}

fn generate_mod_list(path: &Path) -> Vec<mod_information::ModInfo> {
    let mod_reg = Regex::new("\"mod/[^\"]*\"").unwrap();
    let mut settings = path.to_path_buf();
    settings.push("./settings.txt");
    let enabled_mods = fgrep(&settings,&mod_reg,true);
    if enabled_mods.is_empty() {
        eprintln!("Had an issue reading the settings file.");
        return Vec::new();
    }
    let mut mods = Vec::new();
    for i in enabled_mods {
        let mod_file: PathBuf = PathBuf::from(trim_quotes(&i));
        let smod = generate_single_mod(&path,&mod_file);
        if let Some(good_mod) = smod {
            mods.push(good_mod);
        }
    }
    mods
}