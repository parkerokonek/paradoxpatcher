mod moddata;
mod merge_diff;
mod io;

pub use moddata::{mod_info::ModInfo,mod_pack::ModPack};

use std::path::{PathBuf,Path};
use std::fs::{self,File};
use std::io::{prelude::*,BufReader};
use std::collections::HashMap;

use serde::Deserialize;
use lazy_static::lazy_static;
use regex::Regex;

use zip::read::ZipArchive;

use merge_diff::diff_single_conflict;

use io::{files,zips};

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
}

#[derive(Deserialize,Debug)]
struct ConfigListItem {
    datapath: String,
    modpath: String,
    valid_paths: Vec<String>,
    valid_extensions: Vec<String>,
}

impl From<(String,ConfigListItem)> for ConfigOptions {
    fn from(from_tuple: (String, ConfigListItem)) -> Self {
        let valid_paths: Vec<PathBuf> = from_tuple.1.valid_paths.iter().map(|x| PathBuf::from(&x)).collect();
        let valid_extensions: Vec<String> = from_tuple.1.valid_extensions;
        ConfigOptions {game_name: from_tuple.0, mod_path: PathBuf::from(from_tuple.1.modpath), data_path: PathBuf::from(from_tuple.1.datapath), valid_paths, valid_extensions}
    }
}

lazy_static! {
    // Evaluate all of our regular expressions just once for efficiency and things only dying the first time
    static ref RE_DEPS: Regex     = Regex::new(r#"(?m)dependencies[^}]+"#).unwrap();
    static ref RE_SING: Regex     = Regex::new(r#""[^"]+""#).unwrap();
    static ref RE_ARCHIVE: Regex  = Regex::new(r#"archive\s*=\s*"[^"]*\.zip""#).unwrap();
    static ref RE_PATHS: Regex    = Regex::new(r#"path\s*=\s*"[^"]*""#).unwrap();
    static ref RE_NAMES: Regex    = Regex::new(r#"name\s*=\s*"[^"]*""#).unwrap();
    static ref RE_REPLACE: Regex  = Regex::new(r#"replace_path\s*=\s*"[^"]*""#).unwrap();
    static ref RE_MOD: Regex      = Regex::new("\"mod/[^\"]*\"").unwrap();
    static ref RE_USER_DIR: Regex = Regex::new(r#"user_dir\s*=\s*"[^"]*""#).unwrap();
}
    
pub fn parse_configs(arguments: &ArgOptions) -> Result<ConfigOptions,std::io::Error> {
        let config_file = File::open(&arguments.config_path);
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

    
fn fgrep(file_path: &Path, re: &Regex, all_matches: bool) -> Vec<String> {
        if let Some(input) = files::fetch_file_in_path(file_path,true,true) {
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
            return vec![valid.as_str().to_string()];
        }
        
        Vec::new()
}
    
fn collect_dependencies(mod_path: &Path) -> Vec<String> {
        let results = fgrep(mod_path,&RE_DEPS,false);
        
        if !results.is_empty() {
            let dependencies = results[0].replace(r#"\""#, "").replace("\r","");
            let single_deps = grep(&dependencies,&RE_SING,true);
            let single_deps: Vec<String> = single_deps.iter().map(|x| trim_quotes(x)).collect();
            
            single_deps
        } else {
            Vec::new()
        }
}
    
fn trim_quotes(input: &str) -> String {
        let left: Vec<&str> = input.split('"').collect();
        if left.len() == 3 {
            return left[1].to_string();
        }
        String::new()
}
    
fn generate_single_mod(mod_path: &Path, mod_file: &Path) -> Option<ModInfo> {
        let modmod_path: PathBuf = [mod_path,mod_file].iter().collect();
        let dependencies = collect_dependencies(&modmod_path);
        
        let modmod_content = files::fetch_file_in_path(&modmod_path,true,true).unwrap_or_default();
        
        let archive: Vec<String> = grep(&modmod_content, &RE_ARCHIVE, false).iter().map(|x| trim_quotes(x) ).collect();
        let path: Vec<String> = grep(&modmod_content, &RE_PATHS, false).iter().map(|x| trim_quotes(x)).collect();
        let name: Vec<String> = grep(&modmod_content, &RE_NAMES, false).iter().map(|x| trim_quotes(x)).collect();
        let replace_paths: Vec<PathBuf> = grep(&modmod_content, &RE_REPLACE, true).iter().map(|x| PathBuf::from(trim_quotes(x))).collect();
        let user_dir: Option<String> = grep(&modmod_content, &RE_USER_DIR, false).iter().map(|x| trim_quotes(x)).next();
        
        let path: Vec<String> = path.into_iter().filter(|x| !&replace_paths.contains(&PathBuf::from(&x))).collect();
        
        if archive.is_empty() && path.is_empty() || !archive.is_empty() && !path.is_empty() {
            eprintln!("{}\n Spaghetti-Os", &modmod_path.display());
            eprintln!("{},{}",archive.is_empty(),path.is_empty());
            return None;
        } else if name.len() == 1 && archive.len() == 1 {
            let zip_path: PathBuf = [mod_path.to_str()?,&archive[0]].iter().collect();
            let zip_path = files::find_even_with_case(&zip_path)?;
            let file = match File::open(&zip_path) {
                Ok(f) => f,
                Err(e) => {eprintln!("{}",e); return None},
            };
            let reader = BufReader::new(file);
            let zipfile = match ZipArchive::new(reader) {
                Ok(z) => z,
                Err(e) => {eprintln!("{}",e); return None},
            };
            
            let files: Vec<&str> = zipfile.file_names().collect();
            
            return Some(ModInfo::new(mod_path.to_path_buf(),&files,zip_path,name[0].clone(),&dependencies,&replace_paths,user_dir));
        } else if name.len() == 1 && path.len() == 1 {
            let dir_path: PathBuf = [mod_path.to_str()?,&path[0]].iter().collect();
            let dir_path = files::find_even_with_case(&dir_path)?;
            
            let file_check = files::walk_in_dir(&dir_path,Some(&dir_path));
            let files_ref: Vec<&str> = file_check.iter().map(|x| x.to_str().unwrap_or_default()).collect();
            return Some(ModInfo::new(mod_path.to_path_buf(),&files_ref,dir_path,name[0].clone(),&dependencies,&replace_paths,user_dir));
        }
        
        None
}
    
pub fn generate_mod_list(path: &Path) -> Vec<ModInfo> {
        let mod_reg = Regex::new("\"mod/[^\"]*\"").unwrap();
        let mut settings = path.to_path_buf();
        settings.push("settings.txt");
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
            } else {
                eprintln!("The following mod failed to load:\t{}",mod_file.display());
            }
        }
        mods
}
    
pub fn files_in_vanilla(config: &ConfigOptions) -> Vec<PathBuf> {
        let vanilla_path = &config.data_path;
        let check_paths: Vec<PathBuf> = config.valid_paths.iter().map(|x| [vanilla_path,x].iter().collect()).collect();
        let mut out = Vec::new();
        for i in &check_paths {
            let mut path_vec: Vec<PathBuf> = Vec::new();
            let _bob = |x| {path_vec.push(x)};
            let mut results = files::walk_in_dir(&i,Some(&vanilla_path));
            out.append(&mut results);
        }
        
        if out.is_empty() && check_paths.is_empty() {
            out = files::walk_in_dir(vanilla_path,Some(&vanilla_path));
        }
        
        out
}
    
pub fn auto_merge(config: &ConfigOptions, args: &ArgOptions, mod_pack: &ModPack) -> Result<u32,()> {
    let mut successful = 0;

        for conf in mod_pack.list_conflicts() {
            if args.verbose {
                println!("Attempting to merge: {}",conf.path().display());
            }
            let mut file_contents: Vec<String> = Vec::new();
            let mut file_indices: Vec<usize> = Vec::new();

            let vanilla_file = match vanilla_fetch(conf.path(),config,true,true) {
                Some(contents) => contents,
                None => {
                    eprintln!("Error opening vanilla file for comparison: {}",conf.path().display());
                    return Err(());
                },
            };

            for (idx,mod_info) in conf.list_mods().iter().enumerate() {
                if let Some(current) = mod_pack.get_mod(&mod_info) {
                    if current.is_zip() {
                        if let Some(contents) = mod_zip_fetch(conf.path(), current, true, true) {
                            //println!("{}",contents);
                            file_contents.push(contents);
                            file_indices.push(idx);
                        } else {
                            eprintln!("Error unpacking file in previously registered .zip: {}",mod_info);
                        }
                    } else if let Some(contents) = mod_path_fetch(conf.path(), current,true,true) {
                        //println!("{}",contents);
                        file_contents.push(contents);
                        file_indices.push(idx);
                    } else {
                        eprintln!("Error unpacking file in previously registered folder: {}",mod_info);
                    }
                } else {
                    eprintln!("Was unable to unpack one of the previously read mods: {}",mod_info);
                    return Err(());
                }
            }

            let file_content = diff_single_conflict(&vanilla_file, &file_contents, false);

            if let Some(content) = &file_content {
                let mod_folder = args.folder_name();
                let mod_folder: &Path = Path::new(&mod_folder);

                match write_to_mod_folder_string(mod_folder, content.clone(), conf.path(),true) {
                    Ok(_) => successful+=1,
                    Err(e) => eprintln!("{}",e),
                };
                continue;
            } else {
                eprintln!("This file will need manual merging: {}",conf.path().display());

                //Process vanilla file
                let mod_folder = args.folder_name() + "_bad";
                let cur_folder: PathBuf = [&mod_folder,"vanilla"].iter().collect();
                let _try_write = write_to_mod_folder_string(&cur_folder, vanilla_file, conf.path(), true);

                //Process the rest of the files
                for (file_index,file_content) in file_indices.iter().zip(file_contents) {
                    let cur_mod = &conf.list_mods()[*file_index];
                    let cur_mod = match mod_pack.get_mod(cur_mod) {
                        Some(m) => m,
                        None => return Err(()),
                    };
                    let cur_folder: PathBuf = [&mod_folder,cur_mod.get_name()].iter().collect();
                    let _try_write = write_to_mod_folder_string(&cur_folder, file_content, conf.path(), true);
                }
            }
        }
        
        Ok(successful)
}

fn current_dir_path(_args: &ArgOptions, path: &Path) -> Result<PathBuf,std::io::Error> {
    let current_dir = std::env::current_dir()?;
    Ok(current_dir.join(path))
}

fn write_to_mod_folder(mod_folder: &Path, contents: &[u8], path: &Path, encode: bool) -> Result<(),std::io::Error> {
    let full_path = files::relative_folder_path(mod_folder, &path)?;
    files::write_file_with_content(&full_path, contents)
}

fn write_to_mod_folder_string(mod_folder: &Path, contents: String, path: &Path, encode: bool) -> Result<(),std::io::Error> {
    let full_path = files::relative_folder_path(mod_folder, &path)?;
    files::write_file_with_string(&full_path, contents, encode)
}

fn write_to_mod_zip(mod_folder: &Path, staged_data: HashMap<String,Vec<u8>>, zip: &Path) -> Result<(),std::io::Error> {
    let zip_path = files::relative_folder_path(mod_folder, zip)?;
    zips::zip_write_files(&zip_path,staged_data)
    
}

/// Generates and writes a .mod file for the modpack at the designated location.
/// Uses the dependencies of all conflicting mods, as well as replacement paths and user directories
/// Takes name from Arg Options
/// 
/// # Arguments
/// 
/// * `args` - Program arguments, includes name of mod, data locations, etc.
/// 
/// * `mod_pack` - information on all loaded mods, includes conflicting files, enabled mods, etc.

pub fn write_mod_desc_to_folder(args: &ArgOptions, mod_pack: &ModPack) -> Result<(),std::io::Error> {
    let mut mod_file_name = PathBuf::from(args.folder_name());
    mod_file_name.set_extension("mod");

    let full_path = if args.dry_run || mod_pack.list_conflicts().is_empty() {
        current_dir_path(args, &mod_file_name)?
    } else {
        files::relative_folder_path(Path::new(&args.folder_name()), &mod_file_name)?
    };

    //Write the header of the mod file with name and archive
    let mut file_contents = format!("name = \"{}\"\narchive = \"mod/{}.zip\"\n", args.patch_name, args.folder_name());

    if args.extract {
        let mod_user_dirs = mod_pack.list_user_dirs();
        if !mod_user_dirs.is_empty() {
            let mut user_dir = String::from("user_dir = \"");
            for dir in mod_user_dirs {
                user_dir.push_str(&dir);
            }
            user_dir.push_str("\"\n");

            file_contents.push_str(&user_dir);
        }
    }

    // Write Dependencies into the file
    file_contents.push_str("dependencies = {\n");
    for dep in mod_pack.load_order() {
        let dep_text = format!("\"\\\"{}\\\"\"\n",dep);
        file_contents.push_str(&dep_text);
    }
    file_contents.push_str("}\n");

    // If we're doing a full extraction, then grab all of the replacement paths
    if args.extract {
        for single_mod in mod_pack.list_replacement_paths() {
            let replace_line = format!("replace_path = \"{}\"\n",single_mod.display());
            file_contents.push_str(&replace_line);
        }
    }

    // Get path to write to
    let trimmed_path = match full_path.parent() {
        Some(p) => p,
        None => &full_path,
    };
    let _result = fs::create_dir_all(trimmed_path);
    fs::write(full_path,file_contents)?;
    Ok(())
}

/// Wrapper Function for zip_fetch_file_relative that fetches files from a zip folder relative to the mod directory
/// 
/// # Arguments
/// 
/// * `dir` - Path of the file in the mod zip archive we are fetching
/// 
/// * `mod_entry` - Mod information to use for determining which zip archive
/// 
/// * `decode` - If yes, decode the file from Windows-1252 into Unicode
/// 
/// * `normalize` - If yes, to convert all line endings into Windows style line endings
/// 
fn mod_zip_fetch(dir: &Path, mod_entry: &ModInfo, decode: bool, normalize: bool) -> Option<String> {
        if !mod_entry.is_zip() {
            return None;
        }
        let zip_archive = mod_entry.get_data_path();
        zips::zip_fetch_file_relative(dir,zip_archive,decode,normalize)
}

fn mod_path_fetch_all(mod_entry: &ModInfo) -> HashMap<String,Vec<u8>> {
    if mod_entry.is_zip() {
        HashMap::new()
    } else {
        files::fetch_all_files_in_path(mod_entry.get_data_path())
    }
}

fn mod_zip_fetch_all(mod_entry: &ModInfo) -> HashMap<String,Vec<u8>> {
    if !mod_entry.is_zip() {
        HashMap::new()
    } else {
        zips::zip_fetch_all_files(mod_entry.get_data_path())
    }
    
}

fn mod_path_fetch(dir: &Path, mod_entry: &ModInfo, decode: bool, normalize: bool) -> Option<String> {
        let full_path: PathBuf = [mod_entry.get_data_path(),dir].iter().collect();
        files::fetch_file_in_path(&full_path,decode,normalize)
}
    
fn vanilla_fetch(dir: &Path, config: &ConfigOptions, decode: bool, normalize: bool) -> Option<String> {
        let full_path: PathBuf = config.data_path.join(dir);
        files::fetch_file_in_path(&full_path,decode,normalize)
}

pub fn extract_all_files(mods: &ModPack, args: &ArgOptions, config: &ConfigOptions, to_zip: bool) {
    let mod_folder = args.folder_name();
    let mod_folder = Path::new(&mod_folder);
    if to_zip {
        let zip_target = args.folder_name();
        let zip_target: PathBuf = [&zip_target,".zip"].iter().collect();
        let mut staged_zip_data = HashMap::new();
        for mod_idx in mods.load_order() {
            let mod_info = match mods.get_mod(mod_idx) {
                Some(m) => m,
                None => {eprintln!("Error looking up previously registered mod: {}", mod_idx); continue},
            };
                if mod_info.is_zip() {
                    let files = mod_zip_fetch_all(&mod_info);
                    for (file_path,file_data) in files {
                        let _old_data = staged_zip_data.insert(file_path, file_data);
                    }
                } else {
                    let files = mod_path_fetch_all(&mod_info);
                    for (file_path,file_data) in files {
                        let _old_data = staged_zip_data.insert(file_path, file_data);
                    }
                }
        }
        let result = write_to_mod_zip(mod_folder, staged_zip_data, &zip_target);
    } else {
        for mod_idx in mods.load_order() {
            let mod_info = match mods.get_mod(mod_idx) {
                Some(m) => m,
                None => {eprintln!("Error looking up previously registered mod: {}", mod_idx); continue},
            };
                if mod_info.is_zip() {
                    let files = mod_zip_fetch_all(&mod_info);
                    for (file_path,file_data) in files {
                        let result = write_to_mod_folder(mod_folder, &file_data, Path::new(&file_path),true);
                    }
                } else {
                    let res = files::copy_directory_tree(&mod_info.get_data_path() , mod_folder, true, true);
                    //let files = mod_path_fetch_all(&mod_info);
                    //for (file_path,file_data) in files {
                    //    let result = write_to_mod_folder(mod_folder, &file_data, Path::new(&file_path),true);
                    //}
                }
        }
    }
}