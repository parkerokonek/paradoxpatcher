pub mod configs;
mod error;
mod io;
mod merge_diff;
mod moddata;

pub use moddata::{
    mod_conflict::ModConflict,
    mod_info::{ModBuilder, ModInfo},
    mod_pack::{ModPack, ModStatus, ModToken},
};

use error::{verbose_error, MergerError};

use std::collections::HashMap;
use std::fs::{self, File};
use std::io::BufReader;
use std::mem;
use std::path::{Path, PathBuf};

use lazy_static::lazy_static;
use regex::Regex;

use zip::read::ZipArchive;

use merge_diff::diff_single_conflict;

use configs::{ConfigOptions, MergerSettings};
use io::{files, re, zips};

#[cfg(feature = "parallel")]
use rayon::iter::{IntoParallelIterator, ParallelIterator};

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
    let results = files::fgrep(mod_path, &RE_DEPS, false);

    if !results.is_empty() {
        let dependencies = results[0].replace(r#"\""#, "").replace("\r", "");
        let single_deps = re::grep(&dependencies, &RE_SING, true);
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

    let modmod_content = files::fetch_file_in_path(&modmod_path, true, true).unwrap_or_default();

    let grep_strings = |regex, multiple_res| -> Vec<String> {
        re::grep(&modmod_content, regex, multiple_res)
            .iter()
            .map(|x| re::trim_quotes(x))
            .collect()
    };
    let grep_paths = |regex, multiple_res| -> Vec<PathBuf> {
        re::grep(&modmod_content, regex, multiple_res)
            .iter()
            .map(|x| PathBuf::from(re::trim_quotes(x)))
            .collect()
    };

    let archive: Vec<String> = grep_strings(&RE_ARCHIVE, false);
    let path: Vec<String> = grep_strings(&RE_PATHS, false);
    let name: Vec<String> = grep_strings(&RE_NAMES, false);
    let replace_paths: Vec<PathBuf> = grep_paths(&RE_REPLACE, true);
    let user_dir: Option<String> = grep_strings(&RE_USER_DIR, false).pop();

    let mod_name: String = match name.get(0) {
        Some(m_name) if name.len() == 1 => m_name.clone(),
        _ => return None,
    };

    let (is_archive, mod_data_path): (bool, String) = match (archive.get(0), path.get(0)) {
        (Some(m_archive), None) => (true, m_archive.clone()),
        (None, Some(m_path)) => (false, m_path.clone()),
        _ => return None,
    };

    let data_path = mod_path.join(&mod_data_path);
    let data_path = files::find_even_with_case(&data_path)?;

    let mod_builder = match user_dir {
        Some(u_dir) => ModBuilder::new().with_user_directory(&u_dir),
        None => ModBuilder::new(),
    };

    let mod_builder = mod_builder
        .with_mod_path(mod_file.to_path_buf())
        .with_data_path(data_path.clone())
        .with_name(mod_name)
        .with_dependencies(&dependencies)
        .replace_paths(&replace_paths)
        .enabled();

    if is_archive {
        let zip_file = match File::open(&data_path) {
            Ok(f) => f,
            Err(e) => {
                eprintln!("{}", e);
                return None;
            }
        };

        let reader = BufReader::new(zip_file);
        let zipfile = match ZipArchive::new(reader) {
            Ok(z) => z,
            Err(e) => {
                eprintln!("{}", e);
                return None;
            }
        };

        let files: Vec<&str> = zipfile.file_names().collect();

        Some(mod_builder.file_list(&files).into())
    } else {
        let file_check = files::walk_in_dir(&data_path, Some(&data_path));
        let files_ref: Vec<&str> = file_check
            .iter()
            .map(|x| x.to_str().unwrap_or_default())
            .collect();

        Some(mod_builder.file_list(&files_ref).into())
    }
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
    zips::zip_fetch_file_relative(dir, zip_archive, decode, normalize)
}

/// Produce a map of all relative file paths in a mod file directory and their contents
/// #Arguments
///
/// * `mod_entry` - the mod to read
fn mod_path_fetch_all(mod_entry: &ModInfo) -> HashMap<String, Vec<u8>> {
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
fn mod_zip_fetch_all(mod_entry: &ModInfo) -> HashMap<String, Vec<u8>> {
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
fn mod_path_fetch(
    dir: &Path,
    mod_entry: &ModInfo,
    decode: bool,
    normalize: bool,
) -> Result<String, MergerError> {
    let full_path: PathBuf = [mod_entry.get_data_path(), dir].iter().collect();
    files::fetch_file_in_path(&full_path, decode, normalize)
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
fn vanilla_fetch(
    dir: &Path,
    config: &ConfigOptions,
    decode: bool,
    normalize: bool,
) -> Result<String, MergerError> {
    let full_path: PathBuf = config.data_path.join(dir);
    files::fetch_file_in_path(&full_path, decode, normalize)
}

#[derive(Debug, Clone)]
pub struct ModMerger {
    game_config: Option<ConfigOptions>,
    extract_all: bool,
    verbose: bool,
    dry_run: bool,
    patch_name: String,
    patch_path: PathBuf,
}

impl ModMerger {
    pub fn new(extract_all: bool, patch_name: &str, patch_path: &Path) -> Self {
        ModMerger {
            game_config: None,
            extract_all,
            verbose: false,
            dry_run: false,
            patch_name: patch_name.to_owned(),
            patch_path: patch_path.to_path_buf(),
        }
    }

    pub fn using_config(&self, game_config: ConfigOptions) -> Self {
        ModMerger {
            game_config: Some(game_config),
            extract_all: self.extract_all,
            verbose: self.verbose,
            dry_run: self.dry_run,
            patch_name: self.patch_name.clone(),
            patch_path: self.patch_path.clone(),
        }
    }

    pub fn with_config(self, game_config: ConfigOptions) -> Self {
        let mut new_merger = self;
        new_merger.game_config = Some(game_config);

        new_merger
    }

    pub fn set_config(&mut self, game_config: ConfigOptions) {
        self.game_config = Some(game_config);
    }

    pub fn extract(&mut self, extract: bool) -> bool {
        let mut extract = extract;
        mem::swap(&mut extract, &mut self.extract_all);
        extract
    }

    pub fn extract_toggle(&mut self) {
        self.extract_all = !self.extract_all;
    }

    pub fn set_patch_name(&mut self, new_name: String) -> String {
        let mut new_name = new_name;
        mem::swap(&mut new_name, &mut self.patch_name);
        new_name
    }

    pub fn set_patch_path(&mut self, new_path: PathBuf) -> PathBuf {
        let mut new_path = new_path;
        mem::swap(&mut new_path, &mut self.patch_path);
        new_path
    }

    /// Given the path to a paradox game's user directory, generate a list of all enabled mods and their metadata
    /// #Arguments
    ///
    /// * `register_conflicts` - if true, search and record for mod conflicts as we add mods
    pub fn mod_pack_from_enabled(&self, register_conflicts: bool) -> Result<ModPack, MergerError> {
        //TODO: Make more error types
        let config: &ConfigOptions = match &self.game_config {
            Some(conf) => conf,
            None => return Err(MergerError::UnknownError),
        };
        let path = &config.mod_path;
        let new_launcher = config.new_launcher;

        let enabled_mods = ModMerger::list_enabled_mods(path, new_launcher)?;
        let mut mods = Vec::new();

        for i in enabled_mods {
            let mod_file = PathBuf::from(i);
            let smod = generate_single_mod(&path, &mod_file);
            if let Some(good_mod) = smod {
                mods.push(good_mod);
            } else {
                eprintln!("The following mod failed to load:\t{}", mod_file.display());
            }
        }

        let mut mod_pack = ModPack::default()
            .restrict_paths(&config.valid_paths)
            .restrict_extensions(&config.valid_extensions);

        // TODO: files in vanilla probably isn't a merger function
        let vanilla = ModMerger::files_in_vanilla(&config);
        let val_ref: Vec<&Path> = vanilla.iter().map(|x| x.as_path()).collect();
        mod_pack.register_vanilla(&val_ref);

        mod_pack.add_mods(&mods, register_conflicts, register_conflicts);

        Ok(mod_pack)
    }

    /// Generate a list of all mods, enabled or not
    /// #Arguments
    ///
    /// * `register_conflicts` - if true, search and record for mod conflicts as we add mods
    pub fn mod_pack_from_all(&self, register_conflicts: bool) -> Result<Vec<ModInfo>, MergerError> {
        //TODO: Make more error types
        let config: &ConfigOptions = match &self.game_config {
            Some(conf) => conf,
            None => return Err(MergerError::UnknownError),
        };
        let path = &config.mod_path;
        let new_launcher = config.new_launcher;

        let mut mod_list: Vec<ModInfo> = Vec::new();
        let mod_ext = PathBuf::from("mod");
        let mod_mod_ext = PathBuf::from("mod.mod");
        let enabled_mods: Vec<String> = ModMerger::list_enabled_mods(path, new_launcher)?;

        let mods_path = path.join("mod");
        let s_mod_path = PathBuf::from(path);

        for mod_file in files::list_files_in_dir(&mods_path, &[&mod_ext, &mod_mod_ext], true) {
            let mod_mod = PathBuf::from("mod");
            let mod_file = mod_mod.join(mod_file);
            let s_mod = generate_single_mod(&s_mod_path, &mod_file);
            if let Some(good_mod) = s_mod {
                let enabled = enabled_mods
                    .iter()
                    .any(|file| mod_file == PathBuf::from(file.as_str()));
                mod_list.push(good_mod.with_active_state(enabled));
            } else {
                eprintln!("The following mod failed to load:\t{}", mod_file.display());
            }
        }

        let mut new_pack = ModPack::default()
            .restrict_paths(&config.valid_paths)
            .restrict_extensions(&config.valid_extensions);

        let vanilla = ModMerger::files_in_vanilla(&config);
        let val_ref: Vec<&Path> = vanilla.iter().map(|x| x.as_path()).collect();
        new_pack.register_vanilla(&val_ref);
        new_pack.add_mods(&mod_list, register_conflicts, register_conflicts);

        Ok(mod_list)
    }

    /// Writes our current mod list of enabled and disabled mods to the paradox launcher settings file
    /// #Arguments
    ///
    /// * `new_launcher` - true for new launcher settings format, false for old pdx launcher
    ///  
    /// * `most_list` - a list of all mods, marked for enabled or disabled
    pub fn set_entire_mod_list(
        path: &Path,
        new_launcher: bool,
        mod_list: &[ModStatus],
    ) -> Result<(), Box<dyn std::error::Error>> {
        if new_launcher {
            eprintln!("New Launcher");
            return Err(Box::new(MergerError::UnknownError));
        } else {
            let settings = path.join("settings.txt");
            let old_settings_str = files::fetch_file_in_path(&settings, false, true)?;

            let mod_list = mod_list
                .iter()
                .filter(|item| item.status())
                .map(|item| item.mod_file());

            let split_text: Vec<_> = old_settings_str.splitn(2, "last_mods=\r\n{").collect();
            let (text_head, text_body) = match (split_text.get(0), split_text.get(1)) {
                (Some(s1), Some(s2)) => (s1, s2),
                _ => {
                    eprintln!("Could not split settings on mod list.");
                    return Ok(());
                }
            };

            let text_tail: Vec<&str> = text_body.splitn(2, "}\r\n").collect();

            let (_mod_list, text_tail) = match (text_tail.get(0), text_tail.get(1)) {
                (Some(s1), Some(s2)) => (s1, s2),
                _ => {
                    eprintln!("Could not split settings on mod list.");
                    return Ok(());
                }
            };

            let mut output: String = text_head.to_string();
            output.push_str("last_mods=\r\n{\r\n");

            for item in mod_list {
                if let Some(s) = item.to_str() {
                    output.push('"');
                    output.push_str(&s);
                    output.push_str("\"\r\n");
                }
            }

            output.push_str("}\r\n");
            output.push_str(text_tail);

            files::write_file_with_string(&settings, output, false)?;
        }
        Ok(())
    }

    /// List all Mods that are enabled in the current launcher
    /// #Arguments
    ///
    /// * `path` - Path to paradox user settings folder
    ///
    /// * `new_launcher` - true if using the new Paradox launcher
    fn list_enabled_mods(path: &Path, new_launcher: bool) -> Result<Vec<String>, MergerError> {
        if new_launcher {
            let settings = path.join("dlc_load.json");

            let all_mods_str = files::fetch_file_in_path(&settings, false, false)?;

            // TO DO: Make this simpler, maybe add encapsulation for serde and io errors
            let all_mods: HashMap<String, Vec<String>> = match serde_json::from_str(&all_mods_str) {
                Ok(val) => val,
                Err(e) => return Err(MergerError::LoadFileError(format!("{}", e))),
            };

            match all_mods.get("enabled_mods") {
                Some(enabled) => Ok(enabled.clone()),
                None => Err(MergerError::LoadFileError(
                    "Could not find list of enabled mods in json settings file.".to_string(),
                )),
            }
        } else {
            let settings = path.join("settings.txt");
            let enabled_mods = files::fgrep(&settings, &RE_MOD, true);

            if enabled_mods.is_empty() {
                Err(MergerError::LoadFileError(
                    settings.to_string_lossy().to_string(),
                ))
            } else {
                Ok(enabled_mods.iter().map(|s| re::trim_quotes(s)).collect())
            }
        }
    }

    /// Generate a list of all files in the main game directory (fitting our folder and extension requirements)
    /// #Arguments
    ///
    /// * `config` - configuration data
    pub fn files_in_vanilla(config: &ConfigOptions) -> Vec<PathBuf> {
        let vanilla_path = &config.data_path;
        let check_paths: Vec<PathBuf> = config
            .valid_paths
            .iter()
            .map(|x| [vanilla_path, x].iter().collect())
            .collect();
        let mut out = Vec::new();
        for i in &check_paths {
            let mut path_vec: Vec<PathBuf> = Vec::new();
            let _bob = |x| path_vec.push(x);
            let mut results = files::walk_in_dir(&i, Some(&vanilla_path));
            out.append(&mut results);
        }

        if out.is_empty() && check_paths.is_empty() {
            out = files::walk_in_dir(vanilla_path, Some(&vanilla_path));
        }

        out
    }

    /// Performs an automagical merge of the current list of conflicting mods
    /// This can fail for some files, but those files will be placed in their own directory tree for easy manual merging
    ///
    /// #Arguments
    ///
    /// * `mod_pack` - the current mod load order to be merged
    fn auto_merge(&self, mod_pack: &ModPack) -> Result<u32, error::MergerError> {
        let config: &ConfigOptions = self
            .game_config
            .as_ref()
            .expect("The developer is calling code wrong.");
        let folder_name = self.patch_name.to_ascii_lowercase();

        let mut successful = 0;

        #[cfg(feature = "parallel")]
        let conf_iter = mod_pack.list_conflicts().iter().into_par_iter();
        #[cfg(not(feature = "parallel"))]
        let conf_iter = mod_pack.list_conflicts().iter();

        for conf in conf_iter {
            let should_transcode = match conf.path().extension() {
                Some(ext) => !config
                    .no_transcode
                    .iter()
                    .any(|no_ext| ext == no_ext.as_str()),
                None => false,
            };

            let (vanilla_file, file_contents, file_indices) =
                match self.read_mod_conflict(mod_pack, conf, should_transcode) {
                    Ok((a, b, c)) => (a, b, c),
                    Err(e) => {
                        error::verbose_error(self.verbose, false, e)?;
                        continue;
                    }
                };
            let file_content = diff_single_conflict(&vanilla_file, &file_contents, false);

            if let Some(content) = &file_content {
                let mod_folder = folder_name.clone();
                let mod_folder: &Path = Path::new(&mod_folder);

                match ModMerger::write_to_mod_folder_string(
                    mod_folder,
                    content.clone(),
                    conf.path(),
                    should_transcode,
                ) {
                    Ok(_) => successful += 1,
                    Err(e) => eprintln!(
                        "Error with file: {} ==> {} ..with.. {}",
                        conf.path().display(),
                        mod_folder.display(),
                        e
                    ),
                };
                continue;
            } else {
                if self.verbose {
                    eprintln!(
                        "This file will need manual merging: {}",
                        conf.path().display()
                    );
                }

                //Process vanilla file
                let mod_folder = folder_name.clone() + "_bad";
                let cur_folder: PathBuf = [&mod_folder, "vanilla"].iter().collect();
                let _try_write = ModMerger::write_to_mod_folder_string(
                    &cur_folder,
                    vanilla_file,
                    conf.path(),
                    should_transcode,
                );

                //Process the rest of the files
                for (file_index, file_content) in file_indices.iter().zip(file_contents) {
                    let cur_mod = &conf.list_mods()[*file_index];
                    let cur_mod = match mod_pack.get_mod(cur_mod) {
                        Some(m) => m,
                        None => return Err(MergerError::ModPackLookupError(cur_mod.to_owned())),
                    };
                    let cur_folder: PathBuf = [&mod_folder, cur_mod.get_name()].iter().collect();
                    let _try_write = ModMerger::write_to_mod_folder_string(
                        &cur_folder,
                        file_content,
                        conf.path(),
                        should_transcode,
                    );
                }
            }
        }

        Ok(successful)
    }

    /// Helper function for auto_merge that takes information for one mod conflict and tries to load all relevant files
    ///
    /// #Arguments
    ///
    /// * `mod_pack` - the reference mod_pack to pull file info from
    ///
    /// * `should_transcode` - if yes, automatically convert LATIN1 to Unicode and vice versa

    fn read_mod_conflict(
        &self,
        mod_pack: &ModPack,
        mod_conflict: &ModConflict,
        should_transcode: bool,
    ) -> Result<(String, Vec<String>, Vec<usize>), MergerError> {
        let config: &ConfigOptions = self
            .game_config
            .as_ref()
            .expect("The developer is calling code wrong.");

        if self.verbose {
            println!("Attempting to merge: {}", mod_conflict.path().display());
        }
        let mut file_contents: Vec<String> = Vec::new();
        let mut file_indices: Vec<usize> = Vec::new();

        // TODO: Make this work too
        let vanilla_file = match vanilla_fetch(
            mod_conflict.path(),
            &config,
            should_transcode,
            should_transcode,
        ) {
            Ok(contents) => contents,
            Err(_) => {
                let (vanilla_str, conf_str) =
                    ("vanilla".to_string(), mod_conflict.path().to_string_lossy());
                return Err(MergerError::CouldNotCompareError(
                    vanilla_str,
                    conf_str.to_string(),
                ));
            }
        };

        for (idx, mod_info) in mod_conflict.list_mods().iter().enumerate() {
            let mod_file_content = ModMerger::load_file_from_mod(
                mod_pack,
                mod_conflict.path(),
                &mod_info,
                should_transcode,
                should_transcode,
            );
            match mod_file_content {
                Err(MergerError::LoadRegisteredModError(s)) => {
                    eprintln!("{}", MergerError::LoadRegisteredModError(s))
                }
                Ok(content) => {
                    file_contents.push(content);
                    file_indices.push(idx)
                }
                Err(e) => return Err(e),
            }
        }

        Ok((vanilla_file, file_contents, file_indices))
    }

    /// Helper function that the content of a single file from a singular mod in a mod pack, can do Zip or Folder types
    /// #Arguments
    ///
    /// * `mod_pack` - the reference mod pack we are using for our merge
    ///
    /// * `file_path` - the path of the file (relative to the game data directory) we are loading
    ///
    /// * `mod_name` - the name of the mod we are loading, may replace with token later
    ///
    /// * `decode` - true means try to decode LATIN1 into Unicode for Rust's sake
    ///
    /// * `normalize` - true for normalizing line endings to windows-style
    fn load_file_from_mod(
        mod_pack: &ModPack,
        file_path: &Path,
        mod_name: &str,
        decode: bool,
        normalize: bool,
    ) -> Result<String, error::MergerError> {
        if let Some(current) = mod_pack.get_mod(mod_name) {
            if current.is_zip() {
                mod_zip_fetch(file_path, current, decode, normalize)
                    .ok_or_else(|| MergerError::LoadRegisteredModError(mod_name.to_string()))
            } else {
                mod_path_fetch(file_path, current, decode, normalize)
                    .map_err(|_| MergerError::LoadRegisteredModError(mod_name.to_string()))
            }
        } else {
            Err(MergerError::ModPackLookupError(mod_name.to_string()))
        }
    }

    pub fn merge_and_save(&self, mod_pack: &ModPack) -> Result<u32, error::MergerError> {
        //TODO: Make config unwrapping make sense
        let _config = match &self.game_config {
            Some(c) => c,
            _ => return Err(MergerError::UnknownError),
        };

        if self.extract_all {
            self.extract_all_files(mod_pack, false);
        }

        let _e = self.write_mod_desc_to_folder(mod_pack);
        self.auto_merge(mod_pack)
    }

    /// Convert a relative path to the current directory to an absolute path
    /// Can likely be deprecated
    ///
    /// #Arguments
    ///
    /// * `args` - unused command line arguments
    ///
    /// * `path` - relative path to make absolute
    /// //TODO: KILL THIS
    fn current_dir_path(path: &Path) -> Result<PathBuf, std::io::Error> {
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
    fn write_to_mod_folder(
        mod_folder: &Path,
        contents: &[u8],
        path: &Path,
        _encode: bool,
    ) -> Result<(), std::io::Error> {
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
    fn write_to_mod_folder_string(
        mod_folder: &Path,
        contents: String,
        path: &Path,
        encode: bool,
    ) -> Result<(), std::io::Error> {
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
    fn write_to_mod_zip(
        mod_folder: &Path,
        staged_data: HashMap<String, Vec<u8>>,
        zip: &Path,
    ) -> Result<(), std::io::Error> {
        let zip_path = files::relative_folder_path(mod_folder, zip)?;
        zips::zip_write_files(&zip_path, staged_data)
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

    pub fn write_mod_desc_to_folder(&self, mod_pack: &ModPack) -> Result<(), std::io::Error> {
        let folder_name = self.patch_name.to_ascii_lowercase();
        let mut mod_file_name = PathBuf::from(&folder_name);
        mod_file_name.set_extension("mod");

        let full_path = if self.dry_run || mod_pack.list_conflicts().is_empty() {
            ModMerger::current_dir_path(&mod_file_name)?
        } else {
            files::relative_folder_path(&self.patch_path, &mod_file_name)?
        };

        //Write the header of the mod file with name and archive
        let mut file_contents = format!(
            "name = \"{}\"\narchive = \"mod/{}.zip\"\n",
            self.patch_name,
            &mod_file_name.display()
        );

        if self.extract_all {
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
            let dep_text = format!("\"\\\"{}\\\"\"\n", dep.name());
            file_contents.push_str(&dep_text);
        }
        file_contents.push_str("}\n");

        // If we're doing a full extraction, then grab all of the replacement paths
        if self.extract_all {
            for single_mod in mod_pack.list_replacement_paths() {
                let replace_line = format!("replace_path = \"{}\"\n", single_mod.display());
                file_contents.push_str(&replace_line);
            }
        }

        // Get path to write to
        let trimmed_path = match full_path.parent() {
            Some(p) => p,
            None => &full_path,
        };
        let _result = fs::create_dir_all(trimmed_path);
        fs::write(full_path, file_contents)?;
        Ok(())
    }

    /// Extract all files from all currently enabled mods into the output mod directory
    /// #Arguments
    ///
    /// * `mods` - list of enabled mods to extract/copy
    ///
    /// * `to_zip` - if yes, compress output to zip file, uses a lot of memory as all data is written to disk at once
    fn extract_all_files(&self, mods: &ModPack, to_zip: bool) {
        //TODO: clean up extract all files
        //let config = self.game_config.as_ref().expect("File extraction failed because the developer called something wrong.");
        let destination = &self.patch_path;
        let folder_name = self.patch_name.to_ascii_lowercase();

        let mod_folder_buf = destination.join(&folder_name);
        let mod_folder = mod_folder_buf.as_path();
        if to_zip {
            let zip_target = folder_name;
            let zip_target: PathBuf = [&zip_target, ".zip"].iter().collect();
            let _res = self.extract_all_files_zip(mods, mod_folder, &zip_target);
        } else {
            let _res = self.extract_all_files_folder(mods, mod_folder);
        }
    }

    /// Extracts all files in a mod pack into a selected folder
    /// #Arguments
    ///
    /// * `mods` - the mod pack to extract
    ///
    /// * `destination_folder` - folder path to dump content to
    fn extract_all_files_folder(
        &self,
        mods: &ModPack,
        destination_folder: &Path,
    ) -> Result<(), MergerError> {
        for mod_idx in mods.load_order() {
            if mod_idx.status() {
                let mod_info = match mods.get_mod(mod_idx.name()) {
                    Some(m) => m,
                    None => {
                        eprintln!(
                            "Error looking up previously registered mod: {}",
                            mod_idx.name()
                        );
                        continue;
                    }
                };
                if mod_info.is_zip() {
                    let files = mod_zip_fetch_all(&mod_info);
                    for (file_path, file_data) in files {
                        let _res = ModMerger::write_to_mod_folder(
                            destination_folder,
                            &file_data,
                            Path::new(&file_path),
                            true,
                        );
                    }
                } else {
                    let _res = files::copy_directory_tree(
                        &mod_info.get_data_path(),
                        &destination_folder,
                        true,
                        true,
                    );
                    //let files = mod_path_fetch_all(&mod_info);
                    //for (file_path,file_data) in files {
                    //    let result = write_to_mod_folder(mod_folder, &file_data, Path::new(&file_path),true);
                    //}
                }
            }
        }
        Ok(())
    }

    /// Extracts all files in a mod pack into a selected archive
    /// #Arguments
    ///
    /// * `mods` - the mod pack to extract
    ///
    /// * `destination` - parent folder for zip archive
    ///
    /// * `archive` - name of the zip file (including extension)
    fn extract_all_files_zip(
        &self,
        mods: &ModPack,
        destination: &Path,
        archive: &Path,
    ) -> Result<(), MergerError> {
        let mut staged_zip_data = HashMap::new();
        for mod_idx in mods.load_order() {
            if mod_idx.status() {
                let mod_info = match mods.get_mod(mod_idx.name()) {
                    Some(m) => m,
                    None => {
                        eprintln!(
                            "Error looking up previously registered mod: {}",
                            mod_idx.name()
                        );
                        continue;
                    }
                };
                if mod_info.is_zip() {
                    let files = mod_zip_fetch_all(&mod_info);
                    for (file_path, file_data) in files {
                        let _old_data = staged_zip_data.insert(file_path, file_data);
                    }
                } else {
                    let files = mod_path_fetch_all(&mod_info);
                    for (file_path, file_data) in files {
                        let _old_data = staged_zip_data.insert(file_path, file_data);
                    }
                }
            }
        }
        let _result = ModMerger::write_to_mod_zip(destination, staged_zip_data, &archive);
        Ok(())
    }
}
