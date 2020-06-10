use std::path::{Path,PathBuf};
use std::collections::{HashSet,HashMap};

#[derive(Clone)]
pub struct ModInfo {
    mod_path: PathBuf,
    file_tree: HashSet<String>,
    data_path: PathBuf,
    name: String,
    dependencies: Vec<String>,
    replacement_paths: Vec<PathBuf>,
}

impl ModInfo {
    pub fn new(mod_path: PathBuf, file_list: &[&str], data_path: PathBuf, name: String, dependencies: &[String], replacement_paths: &[PathBuf]) -> ModInfo {
        let file_tree = ModInfo::list_to_tree(file_list);
        ModInfo {mod_path,file_tree,data_path,name,dependencies: dependencies.iter().cloned().collect(),replacement_paths: replacement_paths.iter().cloned().collect()}
    }

    fn list_to_tree(list: &[&str]) -> HashSet<String> {
        let mut set = HashSet::new();
        for item in list {
            set.insert(item.to_string());
        }
        
        set
    }

    pub fn empty(mod_path: PathBuf, data_path: PathBuf, name: String) -> ModInfo {
        let dependencies = Vec::new();
        let replacement_paths = Vec::new();
        ModInfo{mod_path,file_tree: HashSet::new(),data_path,name,dependencies,replacement_paths}
    }

    pub fn is_zip(&self) -> bool {
        let out = self.data_path.extension();
        if let Some(ext) = out {
            ext == "zip"
        } else {
            false
        }
    }

    pub fn get_data_path(&self) -> &Path {
        &self.data_path
    }

    pub fn get_name(&self) -> &str {
        &self.name
    }

    pub fn get_filetree(&self) -> &HashSet<String> {
        &self.file_tree
    }

    pub fn list_dependencies(&self) -> &[String] {
        &self.dependencies
    }
}