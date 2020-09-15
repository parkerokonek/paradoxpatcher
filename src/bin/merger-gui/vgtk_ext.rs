// Thanks to jnetterf

use vgtk::lib::gtk::{ComboBoxTextExt};

pub trait ComboBoxTextExtHelpers: ComboBoxTextExt {
    fn set_items(&self, items: Vec<(Option<String>,String)>);
    fn get_items(&self) -> Vec<(Option<String>,String)>;
}

impl<A> ComboBoxTextExtHelpers for A where A: ComboBoxTextExt {
    fn set_items<'a>(&self, items: Vec<(Option<String>,String)>) {
        self.remove_all();
        for (id,text) in items {
            match id {
                None => self.append(None,&text),
                Some(s) => self.append(Some(&s),&text),
            };
        }
    }

    fn get_items(&self) -> Vec<(Option<String>,String)> {
        Vec::new()
    }
}

pub fn to_string_option<S: AsRef<str>>(old_option: Option<S>) -> Option<String> {
    match old_option {
        None => None,
        Some(gs) => Some(String::from(gs.as_ref())),
    }
}