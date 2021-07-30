// Thanks to jnetterf

use vgtk::lib::gtk::ComboBoxTextExt;

pub trait ComboBoxTextExtHelpers: ComboBoxTextExt {
    fn set_items(&self, items: Vec<(Option<String>, String)>);
    fn get_items(&self) -> Vec<(Option<String>, String)>;
    fn set_selected(&self, id: Option<String>);
    fn get_selected(&self) -> Option<String>;
}

impl<A: vgtk::lib::gtk::ComboBoxExt> ComboBoxTextExtHelpers for A
where
    A: ComboBoxTextExt,
{
    fn set_items(&self, items: Vec<(Option<String>, String)>) {
        self.remove_all();
        for (id, text) in items {
            match id {
                None => self.append(None, &text),
                Some(s) => self.append(Some(&s), &text),
            };
        }
    }

    fn get_items(&self) -> Vec<(Option<String>, String)> {
        Vec::new()
    }

    fn set_selected(&self, id: Option<String>) {
        let _id = match id {
            Some(s) => self.set_active_id(Some(s.as_str())),
            None => self.set_active_id(None),
        };
    }

    fn get_selected(&self) -> Option<String> {
        None
    }
}

// MISC functions

pub fn to_string_option<S: AsRef<str>>(old_option: Option<S>) -> Option<String> {
    match old_option {
        None => None,
        Some(gs) => Some(String::from(gs.as_ref())),
    }
}

pub fn gstring_to_string<S: AsRef<str>>(old_string: S) -> String {
    String::from(old_string.as_ref())
}
