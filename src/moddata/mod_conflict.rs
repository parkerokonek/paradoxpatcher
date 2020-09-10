use std::path::{Path,PathBuf};
use std::collections::{HashSet,HashMap};

use super::mod_info::ModInfo;

#[derive(Clone)]
pub struct ModConflict {
    file_path: PathBuf,
    mod_names: Vec<String>,
}

impl ModConflict {
    pub fn new(path: PathBuf, mods: &[String]) -> ModConflict {
        ModConflict{ file_path: path, mod_names: mods.iter().cloned().collect()}
    }

    pub fn compare_mods(mod_list: &[ModInfo],valid_paths: Option<&Vec<PathBuf>>, valid_extensions: Option<&Vec<String>>) -> Vec<ModConflict> {
        let mut out = Vec::new();
        let mut conflicts: HashMap<String,ModConflict> = HashMap::new();

        for mod_info in mod_list {
            for file_path in mod_info.get_filetree() {
                let mut file_path = file_path.to_string();
                file_path.make_ascii_lowercase();
                if conflicts.get(&file_path).is_some() {
                    conflicts.get_mut(&file_path).unwrap().mod_names.push(mod_info.get_name().to_string());
                } else {
                    let conf = ModConflict::new(PathBuf::from(&file_path),&[mod_info.get_name().to_string()]);
                    let extension = match conf.file_path.extension() {
                        Some(ext) => ext,
                        None => continue,
                    };
                    if let Some(real_valid) = valid_paths {
                        if !real_valid.iter().any(|p| conf.in_folder(p)) {
                            continue;
                        }
                    }
                    if let Some(extensions) = valid_extensions {
                        if !extensions.iter().any(|a| extension == a.as_str()) {
                            continue;
                        }
                    }
                    conflicts.insert(file_path, conf);
                }
            }
        }

        for conf in conflicts {
            if conf.1.is_real() {
                out.push(conf.1);
            }
        }

        out
    }

    fn is_real(&self) -> bool {
        self.mod_names.len() > 1
    }

    pub fn display(&self) {
        println!("{}",self.file_path.display());
        //let components: Vec<std::path::Component> = self.file_path.components().collect();
        //println!("{:?}",components);
        println!("{:?}",self.mod_names);
    }

    pub fn in_folder(&self,folder: &Path) -> bool {
        self.file_path.starts_with(folder)
    }

    pub fn path(&self) -> &Path {
        &self.file_path
    }

    pub fn list_mods(&self) -> &[String] {
        &self.mod_names
    }
}