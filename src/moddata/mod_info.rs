use std::path::{Path,PathBuf};
use std::collections::{HashSet};

#[derive(Clone,Debug)]
pub struct ModInfo {
    mod_path: PathBuf,
    file_tree: HashSet<String>,
    data_path: PathBuf,
    name: String,
    dependencies: Vec<String>,
    replacement_paths: Vec<PathBuf>,
    user_dir: Option<String>,
    enabled: bool,
}

impl ModInfo {
    pub fn new(mod_path: PathBuf, file_list: &[&str], data_path: PathBuf, name: String, dependencies: &[String], replacement_paths: &[PathBuf], user_dir: Option<String>, enabled: bool) -> ModInfo {
        let file_tree = ModInfo::list_to_tree(file_list);
        ModInfo {mod_path,file_tree,data_path,name,dependencies: dependencies.to_vec(),replacement_paths: replacement_paths.to_vec(), user_dir, enabled}
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
        ModInfo{mod_path,file_tree: HashSet::new(),data_path,name,dependencies,replacement_paths,user_dir: None, enabled: false}
    }

    pub fn is_zip(&self) -> bool {
        let out = self.data_path.extension();
        if let Some(ext) = out {
            ext == "zip" || ext == "bin"
        } else {
            false
        }
    }

    pub fn get_mod_path(&self) -> &Path {
        &self.mod_path
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

    pub fn list_replacement_paths(&self) -> &[PathBuf] {
        &self.replacement_paths
    }

    pub fn get_user_dir(&self) -> &Option<String> {
        &self.user_dir
    }

    pub fn enable(&mut self) {
        self.enabled = true;
    }

    pub fn disable(&mut self) {
        self.enabled = false;
    }

    pub fn get_active(&self) -> bool {
        self.enabled
    }

    pub fn toggle(&mut self) {
        self.enabled = !self.enabled;
    }

    pub fn with_active_state(self,enabled: bool) -> Self {
        let mut new_info = self;
        new_info.enabled = enabled;
        new_info
    }
}