mod moddata;
mod merge_diff;
mod io;
pub mod configs;

pub use moddata::{mod_info::ModInfo,mod_pack::ModPack,mod_pack::ModStatus,mod_pack::ModToken};

use std::path::{PathBuf,Path};
use std::fs::{self,File};
use std::io::{BufReader};
use std::collections::HashMap;

use lazy_static::lazy_static;
use regex::Regex;

use zip::read::ZipArchive;

use merge_diff::diff_single_conflict;

use io::{files,zips,re};
use configs::{ArgOptions,ConfigOptions};


lazy_static! {
    // Evaluate all of our regular expressions just once for efficiency and things only dying the first time
    static ref RE_DEPS: Regex     = Regex::new(r#"(?m)dependencies[^}]+"#).unwrap();
    static ref RE_SING: Regex     = Regex::new(r#""[^"]+""#).unwrap();
    static ref RE_ARCHIVE: Regex  = Regex::new(r#"archive\s*=\s*"[^"]*\.(zip|bin)""#).unwrap();
    static ref RE_PATHS: Regex    = Regex::new(r#"[^_]path\s*=\s*"[^"]*""#).unwrap();
    static ref RE_NAMES: Regex    = Regex::new(r#"name\s*=\s*"[^"]*""#).unwrap();
    static ref RE_REPLACE: Regex  = Regex::new(r#"replace_path\s*=\s*"[^"]*""#).unwrap();
    static ref RE_MOD: Regex      = Regex::new("\"mod/[^\"]*\"").unwrap();
    static ref RE_USER_DIR: Regex = Regex::new(r#"user_dir\s*=\s*"[^"]*""#).unwrap();
}

/// Given a path to a Paradox mod description file, generate a list of all its dependencies
/// #Arguments
/// 
/// * `mod_path` - path to a valid mod descriptor file
fn collect_dependencies(mod_path: &Path) -> Vec<String> {
        let results = files::fgrep(mod_path,&RE_DEPS,false);
        
        if !results.is_empty() {
            let dependencies = results[0].replace(r#"\""#, "").replace("\r","");
            let single_deps = re::grep(&dependencies,&RE_SING,true);
            let single_deps: Vec<String> = single_deps.iter().map(|x| re::trim_quotes(x)).collect();
            
            single_deps
        } else {
            Vec::new()
        }
}

/// Attempts to create Mod metadata by reading the mod's file directory and description file
/// #Arguments
/// 
/// * `mod_path` - parent path to both the mod content and mod descriptor file
/// 
/// * `mod_file` - mod descriptor file name and extension
fn generate_single_mod(mod_path: &Path, mod_file: &Path) -> Option<ModInfo> {
        let modmod_path: PathBuf = mod_path.join(mod_file);
        let dependencies = collect_dependencies(&modmod_path);

        let modmod_content = files::fetch_file_in_path(&modmod_path,true,true).unwrap_or_default();
        
        let archive: Vec<String> = re::grep(&modmod_content, &RE_ARCHIVE, false).iter().map(|x| re::trim_quotes(x) ).collect();
        let path: Vec<String> = re::grep(&modmod_content, &RE_PATHS, false).iter().map(|x| re::trim_quotes(x)).collect();
        let name: Vec<String> = re::grep(&modmod_content, &RE_NAMES, false).iter().map(|x| re::trim_quotes(x)).collect();
        let replace_paths: Vec<PathBuf> = re::grep(&modmod_content, &RE_REPLACE, true).iter().map(|x| PathBuf::from(re::trim_quotes(x))).collect();
        let user_dir: Option<String> = re::grep(&modmod_content, &RE_USER_DIR, false).iter().map(|x| re::trim_quotes(x)).next();
        
        //let path: Vec<String> = path.into_iter().filter(|x| !&replace_paths.contains(&PathBuf::from(&x))).collect();
        
        if archive.is_empty() && path.is_empty() || !archive.is_empty() && !path.is_empty() {
            eprintln!("{}\n Spaghetti-Os: {} {}", &modmod_path.display(), archive.is_empty(), path.is_empty());
            None
        } else if name.len() == 1 && archive.len() == 1 {
            let zip_path: PathBuf = mod_path.join(&archive[0]);

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
            
            Some(ModInfo::new(mod_file.to_path_buf(),&files,zip_path,name[0].clone(),&dependencies,&replace_paths,user_dir,true))
        } else if name.len() == 1 && path.len() == 1 {
            let dir_path: PathBuf = mod_path.join(&path[0]);
            let dir_path = files::find_even_with_case(&dir_path)?;
            let file_check = files::walk_in_dir(&dir_path,Some(&dir_path));
            let files_ref: Vec<&str> = file_check.iter().map(|x| x.to_str().unwrap_or_default()).collect();
            Some(ModInfo::new(mod_file.to_path_buf(),&files_ref,dir_path,name[0].clone(),&dependencies,&replace_paths,user_dir,true))
        } else {
            None
        }
}

/// Given the path to a paradox game's user directory, generate a list of all enabled mods and their metadata
/// #Arguments
/// 
/// * `path` - Path of the game's user directory, typically in Documents or ~/.Paradox\ Interactive/
pub fn generate_enabled_mod_list(path: &Path, new_launcher: bool) -> Vec<ModInfo> {
    let enabled_mods = list_enabled_mods(path,new_launcher);
    let mut mods = Vec::new();
    
    for i in enabled_mods {
        let mod_file = PathBuf::from(i);
        let smod = generate_single_mod(&path,&mod_file);
        if let Some(good_mod) = smod {
            mods.push(good_mod);
        } else {
            eprintln!("The following mod failed to load:\t{}",mod_file.display());
        }
    }

    mods
}

/// Generate a list of all mods, enabled or not
pub fn generate_entire_mod_list(path: &Path, new_launcher: bool) -> Vec<ModInfo> {
    let mut mod_list: Vec<ModInfo> = Vec::new();
    let mod_ext = PathBuf::from("mod");
    let mod_mod_ext = PathBuf::from("mod.mod");
    let enabled_mods: Vec<String> = list_enabled_mods(path, new_launcher);

    let mods_path = path.join("mod");
    let s_mod_path = PathBuf::from(path);

    for mod_file in files::list_files_in_dir(&mods_path,&[&mod_ext,&mod_mod_ext],true) {
        let mod_mod = PathBuf::from("mod");
        let mod_file = mod_mod.join(mod_file);
        let s_mod = generate_single_mod(&s_mod_path, &mod_file);
        if let Some(good_mod) = s_mod {

            let enabled = enabled_mods.iter().any(|file| mod_file == PathBuf::from(file.as_str()));
            mod_list.push(good_mod.with_active_state(enabled));
        } else {
            eprintln!("The following mod failed to load:\t{}",mod_file.display());
        }
    }

    mod_list
}

pub fn set_entire_mod_list(path: &Path, new_launcher: bool, mod_list: &[ModStatus]) -> Result<(),Box<dyn std::error::Error>> {
    if new_launcher {
        println!("New Launcher");
    } else {
        let settings = path.join("settings.txt");
        let old_settings_str = match files::fetch_file_in_path(&settings, false, true) {
            Some(s) => s,
            None => {
                eprintln!("Had an issue finding and reading the settings file.");
                return Ok(());
            },
        };

        let mod_list = mod_list.iter().filter(|item| item.status() ).map(|item| item.mod_file() );

        let chunks: Vec<_> = old_settings_str.splitn(2,"last_mods=\r\n{").collect();
        if chunks.len() != 2 {
            eprintln!("Wrong number of chunks");
            return Ok(());
        }

        let chunks_boogaloo: Vec<_> = chunks[1].splitn(2,"}\r\n").collect();

        if chunks_boogaloo.len() != 2 {
            eprintln!("Wrong number of boogaloo chunks");
            return Ok(());
        }

        let mut output: String = chunks[0].to_string();
        output.push_str("last_mods=\r\n{\r\n");

        for item in mod_list {
            if let Some(s) = item.to_str() {
                output.push('"');
                output.push_str(&s);
                output.push_str("\"\r\n");
            }
        }

        output.push_str("}\r\n");
        output.push_str(chunks_boogaloo[1]);

        let res = files::write_file_with_string(&settings, output, false)?;

    }
return Ok(())
}

fn list_enabled_mods(path: &Path, new_launcher: bool) -> Vec<String> {
    if new_launcher {
        let settings = path.join("dlc_load.json");

        let all_mods_str = match files::fetch_file_in_path(&settings, false, false) {
            Some(s) => s,
            None => {eprintln!("Had an issue finding and reading the settings file."); return Vec::new()},
        };
        let all_mods: HashMap<String,Vec<String>> = match serde_json::from_str(&all_mods_str) {
            Ok(val) => val,
            Err(e) => {eprintln!("{:?}",e); return Vec::new()},
        };

        match all_mods.get("enabled_mods") {
            Some(enabled) => enabled.clone(),
            None => Vec::new(),
        }
    } else {
        let settings = path.join("settings.txt");
        let enabled_mods = files::fgrep(&settings,&RE_MOD,true);

        if enabled_mods.is_empty() {
            eprintln!("Had an issue reading the settings file.");
            Vec::new()
        } else {
            enabled_mods.iter().map(|s| re::trim_quotes(s)).collect()
        }
    }
}

/// Generate a list of all files in the main game directory (fitting our folder and extension requirements)
/// #Arguments
/// 
/// * `config` - configuration data
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

/// Performs an automagical merge of the current list of conflicting mods
/// This can fail for some files, but those files will be placed in their own directory tree for easy manual merging
/// 
/// #Arguments
/// 
/// * `config` - configuration options for our game
/// 
/// * `args` - options left over from arguments, will be removed soon
/// 
/// * `mod_pack` - the current mod load order to be merged 
pub fn auto_merge(config: &ConfigOptions, args: &ArgOptions, mod_pack: &ModPack) -> Result<u32,()> {
    let mut successful = 0;

        for conf in mod_pack.list_conflicts() {
            if args.verbose {
                println!("Attempting to merge: {}",conf.path().display());
            }
            let mut file_contents: Vec<String> = Vec::new();
            let mut file_indices: Vec<usize> = Vec::new();
            let should_transcode = match conf.path().extension() {
                Some(ext) => !config.no_transcode.iter().any(|no_ext| ext == no_ext.as_str()),
                None => false,
            };

            let vanilla_file = match vanilla_fetch(conf.path(),config,should_transcode,should_transcode) {
                Some(contents) => contents,
                None => {
                    eprintln!("Error opening vanilla file for comparison: {}",conf.path().display());
                    return Err(());
                },
            };

            for (idx,mod_info) in conf.list_mods().iter().enumerate() {
                if let Some(current) = mod_pack.get_mod(&mod_info) {
                    if current.is_zip() {
                        if let Some(contents) = mod_zip_fetch(conf.path(), current, should_transcode, should_transcode) {
                            //println!("{}",contents);
                            file_contents.push(contents);
                            file_indices.push(idx);
                        } else {
                            eprintln!("Error unpacking file in previously registered .zip: {}",mod_info);
                        }
                    } else if let Some(contents) = mod_path_fetch(conf.path(), current,should_transcode,should_transcode) {
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

                match write_to_mod_folder_string(mod_folder, content.clone(), conf.path(), should_transcode) {
                    Ok(_) => successful+=1,
                    Err(e) => eprintln!("Error with file: {} ==> {} ..with.. {}",conf.path().display(),mod_folder.display(),e),
                };
                continue;
            } else {
                eprintln!("This file will need manual merging: {}",conf.path().display());

                //Process vanilla file
                let mod_folder = args.folder_name() + "_bad";
                let cur_folder: PathBuf = [&mod_folder,"vanilla"].iter().collect();
                let _try_write = write_to_mod_folder_string(&cur_folder, vanilla_file, conf.path(), should_transcode);

                //Process the rest of the files
                for (file_index,file_content) in file_indices.iter().zip(file_contents) {
                    let cur_mod = &conf.list_mods()[*file_index];
                    let cur_mod = match mod_pack.get_mod(cur_mod) {
                        Some(m) => m,
                        None => return Err(()),
                    };
                    let cur_folder: PathBuf = [&mod_folder,cur_mod.get_name()].iter().collect();
                    let _try_write = write_to_mod_folder_string(&cur_folder, file_content, conf.path(), should_transcode);
                }
            }
        }
        
        Ok(successful)
}

/// Convert a relative path to the current directory to an absolute path
/// Can likely be deprecated
/// 
/// #Arguments
/// 
/// * `args` - unused command line arguments
/// 
/// * `path` - relative path to make absolute
fn current_dir_path(_args: &ArgOptions, path: &Path) -> Result<PathBuf,std::io::Error> {
    let current_dir = std::env::current_dir()?;
    Ok(current_dir.join(path))
}

/// Write a byte buffer to a file in a mod folder
/// 
/// #Arguments
/// 
/// * `mod_folder` - mod parent directory, typically for merged mod
/// 
/// * `contents` - bytes to write into the file
/// 
/// * `path` - relative file path in the parent directory
/// 
/// * `encode` - if yes, encode in WINDOWS-1252, otherwise write as-is
fn write_to_mod_folder(mod_folder: &Path, contents: &[u8], path: &Path, _encode: bool) -> Result<(),std::io::Error> {
    let full_path = files::relative_folder_path(mod_folder, &path)?;
    files::write_file_with_content(&full_path, contents)
}


/// Write a string to a file in a mod folder
/// 
/// #Arguments
/// 
/// * `mod_folder` - mod parent directory, typically for merged mod
/// 
/// * `contents` - string to write into the file
/// 
/// * `path` - relative file path in the parent directory
/// 
/// * `encode` - if yes, encode in WINDOWS-1252, otherwise write as-is
fn write_to_mod_folder_string(mod_folder: &Path, contents: String, path: &Path, encode: bool) -> Result<(),std::io::Error> {
    let full_path = files::relative_folder_path(mod_folder, &path)?;
    files::write_file_with_string(&full_path, contents, encode)
}

/// Write a set of files and their byte contents to a new zip file, will overwrite an existing file
/// 
/// #Arguments
/// 
/// * `mod_folder` - parent directory for zip file
/// 
/// * `staged_data` - list of file names and associated data
/// 
/// * `zip` - zip filename
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
        let dep_text = format!("\"\\\"{}\\\"\"\n",dep.name());
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

/// Produce a map of all relative file paths in a mod file directory and their contents
/// #Arguments
/// 
/// * `mod_entry` - the mod to read
fn mod_path_fetch_all(mod_entry: &ModInfo) -> HashMap<String,Vec<u8>> {
    if mod_entry.is_zip() {
        HashMap::new()
    } else {
        files::fetch_all_files_in_path(mod_entry.get_data_path())
    }
}

/// Produce a map of all relative file paths in a mod zip file and their contents
/// #Arguments
/// 
/// * `mod_entry` - the mod to read
fn mod_zip_fetch_all(mod_entry: &ModInfo) -> HashMap<String,Vec<u8>> {
    if !mod_entry.is_zip() {
        HashMap::new()
    } else {
        zips::zip_fetch_all_files(mod_entry.get_data_path())
    }
    
}

/// Get the contents of a single file in a mod file directory or zip file
/// #Arguments
/// 
/// * `dir` - file to extact data from
/// 
/// * `mod_entry` - mod to extract from
/// 
/// * `decode` - if yes, attempt to decode the file contents, otherwise read in bytes as-is
/// 
/// * `normalize` - if yes, convert all line-endings to windows-style
fn mod_path_fetch(dir: &Path, mod_entry: &ModInfo, decode: bool, normalize: bool) -> Option<String> {
        let full_path: PathBuf = [mod_entry.get_data_path(),dir].iter().collect();
        files::fetch_file_in_path(&full_path,decode,normalize)
}

/// Get the contents of a single file in a vanilla file directory
/// #Arguments
/// 
/// * `dir` - file to extact data from
/// 
/// * `config` - information about game directories
/// 
/// * `decode` - if yes, attempt to decode the file contents, otherwise read in bytes as-is
/// 
/// * `normalize` - if yes, convert all line-endings to windows-style
fn vanilla_fetch(dir: &Path, config: &ConfigOptions, decode: bool, normalize: bool) -> Option<String> {
        let full_path: PathBuf = config.data_path.join(dir);
        files::fetch_file_in_path(&full_path,decode,normalize)
}

/// Extract all files from all currently enabled mods into the output mod directory
/// #Arguments
/// 
/// * `mods` - list of enabled mods to extract/copy
/// 
/// * `args` - basic output mod info
/// 
/// * `config` - information about the game files
/// 
/// * `to_zip` - if yes, compress output to zip file, uses a lot of memory as all data is written to disk at once
pub fn extract_all_files(mods: &ModPack, args: &ArgOptions, _config: &ConfigOptions, to_zip: bool, destination: &Path) {
    let mod_folder_buf = destination.join(args.folder_name());
    let mod_folder = mod_folder_buf.as_path();
    if to_zip {
        let zip_target = args.folder_name();
        let zip_target: PathBuf = [&zip_target,".zip"].iter().collect();
        let mut staged_zip_data = HashMap::new();
        for mod_idx in mods.load_order() {
            if mod_idx.status() {
            let mod_info = match mods.get_mod(mod_idx.name()) {
                Some(m) => m,
                None => {eprintln!("Error looking up previously registered mod: {}", mod_idx.name()); continue},
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
        }
        let _result = write_to_mod_zip(mod_folder, staged_zip_data, &zip_target);
    } else {
        for mod_idx in mods.load_order() {
            if mod_idx.status() {
            let mod_info = match mods.get_mod(mod_idx.name()) {
                Some(m) => m,
                None => {eprintln!("Error looking up previously registered mod: {}", mod_idx.name()); continue},
            };
                if mod_info.is_zip() {
                    let files = mod_zip_fetch_all(&mod_info);
                    for (file_path,file_data) in files {
                        let _result = write_to_mod_folder(mod_folder, &file_data, Path::new(&file_path),true);
                    }
                } else {
                    let _res = files::copy_directory_tree(&mod_info.get_data_path() , &mod_folder, true, true);
                    //let files = mod_path_fetch_all(&mod_info);
                    //for (file_path,file_data) in files {
                    //    let result = write_to_mod_folder(mod_folder, &file_data, Path::new(&file_path),true);
                    //}
                }
        }
    }
    }
}