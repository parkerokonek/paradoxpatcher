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

pub struct ModBuilder {
    mod_info: ModInfo
}

impl Default for ModInfo {
    fn default() -> Self {
        ModInfo {
            mod_path: PathBuf::new(),
            file_tree: HashSet::new(),
            data_path: PathBuf::new(),
            name: String::new(),
            dependencies: Vec::new(),
            replacement_paths: Vec::new(),
            user_dir: None,
            enabled: false,
        }
    }
}

impl ModInfo {
    fn list_to_tree(list: &[&str]) -> HashSet<String> {
        let mut set = HashSet::new();
        for item in list {
            set.insert(item.to_string());
        }
        
        set
    }

    pub fn new(mod_path: PathBuf, data_path: PathBuf, name: String) -> ModInfo {
        let mut mod_info: ModInfo = ModInfo::default();
        mod_info.mod_path = mod_path;
        mod_info.data_path = data_path;
        mod_info.name = name;
        mod_info
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

impl ModBuilder {
    pub fn new() -> Self {
        ModBuilder {mod_info: ModInfo::default()}
    }

    pub fn with_name(mut self, name: String) -> Self {
        self.mod_info.name = name;
        self
    }

    pub fn file_list(mut self, file_list: &[&str]) -> Self {
        let file_tree = ModInfo::list_to_tree(file_list);
        self.mod_info.file_tree = file_tree;
        self
    }

    pub fn enabled(mut self) -> Self {
        self.mod_info.enabled = true;
        self
    }

    pub fn disabled(mut self) -> Self {
        self.mod_info.enabled = false;
        self
    }

    pub fn with_dependencies(mut self, dependencies: &[String]) -> Self {
        self.mod_info.dependencies = dependencies.to_vec();
        self
    }

    pub fn replace_paths(mut self, replacements: &[PathBuf]) -> Self {
        self.mod_info.replacement_paths = replacements.to_vec();
        self
    }

    pub fn with_user_directory(mut self, user_dir: &str) -> Self {
        self.mod_info.user_dir = Some(user_dir.to_owned());
        self
    }

    pub fn with_mod_path(mut self, path: PathBuf) -> Self {
        self.mod_info.mod_path = path;
        self
    }

    pub fn with_data_path(mut self, path: PathBuf) -> Self {
        self.mod_info.data_path = path;
        self
    }

    pub fn finish(&self) -> ModInfo {
        self.mod_info.clone()
    }
}

impl From<ModBuilder> for ModInfo {
    fn from(mod_builder: ModBuilder) -> Self {
        mod_builder.mod_info
    }
}