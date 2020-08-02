use diff_match_patch::{Dmp,Diff,Patch};

fn diff_by_lines(base_text: &str, modified_text: &str) -> Vec<Diff> {
    let mut dmp = Dmp::new();
    let (text_1,text_2,line_array) = dmp.diff_lines_tochars(&base_text.chars().collect(), &modified_text.chars().collect());

    let mut diffs = dmp.diff_main(&text_1, &text_2, false);

    dmp.diff_chars_tolines(&mut diffs, &line_array);
    dmp.diff_cleanup_semantic(&mut diffs);

    diffs
}

pub fn diff_single_conflict(base_text: &str, modded_texts: &[String], _verbose: bool) -> Option<String> {
    let mut dmp = Dmp::new();
    // 0.27 is the magic number to prevent patches overriding each other
    dmp.match_threshold = 0.27;

    let mut patch_list: Vec<Vec<Patch>> = modded_texts.iter().map( |mod_text| {
        dmp.patch_make4(base_text, &mut diff_by_lines(base_text, mod_text))
    }).collect();

    // Make the smaller patches come first, for no reason other than standard patch order
    patch_list.sort_by_key(|patch| patch.len());

    let mut result_text: String = base_text.to_owned();
    for patch in &mut patch_list {
        let (changed_text,applied_patches) = dmp.patch_apply(patch, &result_text);

        if applied_patches.iter().fold(true, |a,b| a & b) {
            result_text = changed_text.iter().collect();
        } else {
            return None;
        }
    }

    Some(result_text)
}

#[cfg(test)]
mod test {
    use super::*;
    #[test]
    fn test_diff_two_line_changes() {
        let source = "OR = \r\n{\r\n\ttier = KING\r\n\tcash = 240\r\n\treligion = rustacean\r\n}\r\n".to_owned();
        let change_tier = "OR = \r\n{\r\n\ttier = COUNT\r\n\tcash = 240\r\n\treligion = rustacean\r\n}\r\n".to_owned();
        let change_cash = "OR = \r\n{\r\n\ttier = KING\r\n\teggs = 89\r\n\treligion = rustacean\r\n}\r\n".to_owned();
        
        let both_changes = "OR = \r\n{\r\n\ttier = COUNT\r\n\teggs = 89\r\n\treligion = rustacean\r\n}\r\n".to_owned();
        
        assert_eq!(diff_single_conflict(&source, &[change_tier,change_cash], false),Some(both_changes));
    }
    
    #[test]
    fn test_diff_two_removals() {
        let source = "OR = \r\n{\r\n\ttier = KING\r\n\tcash = 240\r\n\treligion = rustacean\r\n}\r\n".to_owned();
        let no_tier = "OR = \r\n{\r\n\tcash = 240\r\n\treligion = rustacean\r\n}\r\n".to_owned();
        let no_cash = "OR = \r\n{\r\n\ttier = KING\r\n\treligion = rustacean\r\n}\r\n".to_owned();
        
        let both_changes = "OR = \r\n{\r\n\treligion = rustacean\r\n}\r\n".to_owned();
        
        assert_eq!(diff_single_conflict(&source, &[no_tier,no_cash], false),Some(both_changes));
    }
    
    #[test]
    fn test_diff_remove_first() {
        let source = "OR = \r\n{\r\n\ttier = KING\r\n\tcash = 240\r\n\treligion = rustacean\r\n}\r\n".to_owned();
        let no_tier = "OR = \r\n{\r\n\tcash = 240\r\n\treligion = rustacean\r\n}\r\n".to_owned();
        let change_cash = "OR = \r\n{\r\n\ttier = KING\r\n\t AND = {\r\n\tbob = jim\r\n\t zoop = zorp}\r\n\treligion = rustacean\r\n}\r\n".to_owned();
        
        let both_changes = "OR = \r\n{\r\n\t AND = {\r\n\tbob = jim\r\n\t zoop = zorp}\r\n\treligion = rustacean\r\n}\r\n".to_owned();
        
        assert_eq!(diff_single_conflict(&source, &[no_tier,change_cash], false),Some(both_changes));
    }
    
    #[test]
    fn test_two_conflicting_changes() {
        let source = "OR = \r\n{\r\n\ttier = KING\r\n\tcash = 240\r\n\treligion = rustacean\r\n}\r\n".to_owned();
        let change_tier_1 = "OR = \r\n{\r\n\ttier = DUKE\r\n\tcash = 240\r\n\treligion = rustacean\r\n}\r\n".to_owned();
        let change_tier_2 = "OR = \r\n{\r\n\ttier = COUNT\r\n\tcash = 240\r\n\treligion = rustacean\r\n}\r\n".to_owned();
        
        assert_eq!(diff_single_conflict(&source, &[change_tier_1,change_tier_2], false),None);
    }
    
    
    #[test]
    fn test_three_conflicting_changes() {
        let source = "OR = \r\n{\r\n\ttier = KING\r\n\tcash = 240\r\n\treligion = rustacean\r\n}\r\n".to_owned();
        let change_tier_1 = "OR = \r\n{\r\n\ttier = DUKE\r\n\tcash = 240\r\n\treligion = rustacean\r\n}\r\n".to_owned();
        let change_tier_2 = "OR = \r\n{\r\n\ttier = COUNT\r\n\tcash = 240\r\n\treligion = rustacean\r\n}\r\n".to_owned();
        let change_tier_3 = "OR = \r\n{\r\n\ttier = EMPEROR\r\n\tcash = 240\r\n\treligion = rustacean\r\n}\r\n".to_owned();
        
        assert_eq!(diff_single_conflict(&source, &[change_tier_1,change_tier_2,change_tier_3], false),None);
    }
    
    #[test]
    fn test_conflicting_character_changes() {
        let source = "OR = \r\n{\r\n\ttier = KING\r\n\tcash = 240\r\n\treligion = rustacean\r\n}\r\n".to_owned();
        let source_a = "OR = \r\n{\r\n\ttier = KANG\r\n\tcash = 240\r\n\treligion = rustacean\r\n}\r\n".to_owned();
        let source_o = "OR = \r\n{\r\n\ttier = KONG\r\n\tcash = 240\r\n\treligion = rustacean\r\n}\r\n".to_owned();
        
        // Result should be a failure
        assert_eq!(diff_single_conflict(&source,&[source_a.clone(),source_o.clone()],false),None, "Conflicting entries on a line, even by a single character, should not produce output text.");
        // Result should not depend on patch order if it will fail
        assert_eq!(diff_single_conflict(&source,&[source_a.clone(),source_o.clone()],false),diff_single_conflict(&source,&[source_o.clone(),source_a.clone()],false),"Result changes based off of the order of the changed files.");
    }

    #[test]
    fn test_adding_many_lines() {
        let source = "this is some\r\ntext that will have many lines added in the middle with\r\na lot of content";
        let big_change = "this is some\r\nAAAAAAAAAAAAAAAAASJHljshdflshjdfhsdjfhjsdhfljshflsdfhsd\r\na lot of\r\nsdkjfhskjdhfkjshdfkjnsdjfhkjsdhfuhsdfhljsdfljshdfjsdfhljsdhfljsdhfysdofh      \t   \r\ntext that will have many lines added in the middle with\r\na lot of content".to_owned();
        let small_change_later = "this is some\r\ntext that will have many lines added in the middle with\r\na lot of changes in the middle that makes a lot of gibberish the content".to_owned();

        let result = "this is some\r\nAAAAAAAAAAAAAAAAASJHljshdflshjdfhsdjfhjsdhfljshflsdfhsd\r\na lot of\r\nsdkjfhskjdhfkjshdfkjnsdjfhkjsdhfuhsdfhljsdfljshdfjsdfhljsdhfljsdhfysdofh      \t   \r\ntext that will have many lines added in the middle with\r\na lot of changes in the middle that makes a lot of gibberish the content".to_owned();

        assert_eq!(diff_single_conflict(source, &[big_change.clone(),small_change_later.clone()], false),Some(result.clone()));
        assert_eq!(diff_single_conflict(source, &[small_change_later,big_change], false),Some(result));
    }
}