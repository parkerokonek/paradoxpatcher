use std::{error::Error, fmt};

#[derive(Debug)]
pub enum MergerError {
    SplitSettingsError,
    LoadFileError(String),
    FindSettingsError,
    MainFileMissingError(String),
    LoadFolderError(String),
    LoadZipError(String),
    LoadRegisteredModError(String),
    ReadDescriptorError,
    CouldNotCompareError(String,String),
    ModPackLookupError(String),
    UnknownError,
}

impl MergerError {
    fn error_text(&self) -> String {
        match self {
            MergerError::SplitSettingsError => String::from("Unable to split settings file on loaded mod list."),
            MergerError::LoadFileError(s) => format!("Could not load the specified file: {}", s),
            MergerError::FindSettingsError => String::from("Could not locate the launcher settings file at the specified directory."),
            MergerError::MainFileMissingError(s) => format!("Could not load enabled mod. File does not exist. {}", s),
            MergerError::LoadFolderError(s) => format!("Mod data folder does not exist. {}", s),
            MergerError::LoadZipError(s) => format!("Mod data archive does not exist. {}", s),
            MergerError::LoadRegisteredModError(s) => format!("Could not load files for previously registered mod. {}", s),
            MergerError::ReadDescriptorError => String::from("Unable to read .mod descriptor file."),
            MergerError::ModPackLookupError(s) => format!("Unable to retrieve information on requested mod. {}", s),
            MergerError::CouldNotCompareError(s1,s2) => format!("Could not load {} file for comparision. {}", s1, s2),
            _ => String::from("Unknown Merger Error"),
        }
    }

    fn variant_name(&self) -> String {
        match self {
            MergerError::SplitSettingsError => "Split Settings Error",
            MergerError::LoadFileError(_) => "Load File Error",
            MergerError::FindSettingsError => "Find Settings Error",
            MergerError::MainFileMissingError(_) => "Main File Missing Error",
            MergerError::LoadFolderError(_) => "Load Folder Error",
            MergerError::LoadZipError(_) => "Load Zip Error",
            MergerError::LoadRegisteredModError(_) => "Load Registered Mod Error",
            MergerError::ReadDescriptorError => "Read Descriptor Error",
            MergerError::CouldNotCompareError(_,_) => "Could Not Compare Error",
            MergerError::ModPackLookupError(_) => "Modpack Lookup Error",
            MergerError::UnknownError => "Unknown Error",
        }.to_string()
    }
}

impl fmt::Display for MergerError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f,"{e}: {t}", e = self.variant_name(), t = self.error_text())
    }
}

impl Error for MergerError {}