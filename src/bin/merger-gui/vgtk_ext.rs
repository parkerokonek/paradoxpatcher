// Thanks to jnetterf

use vgtk::lib::gtk::{ComboBoxText,ComboBoxTextExt};

pub trait ComboBoxTextExtHelpers: ComboBoxTextExt {
    fn set_items(&self, items: Vec<(Option<String>,String)>);
    fn get_items(&self) -> Vec<(Option<String>,String)>;
    fn get_active_text(&self) -> Option<String>;
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

    fn get_active_text(&self) -> Option<String> {
        match self.get_active_text() {
            None => None,
            Some(g_string) => Some(String::from(g_string.as_str())),
        }
    }
}