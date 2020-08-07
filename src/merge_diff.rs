use diff_match_patch::{Dmp,Diff,Patch};
use std::collections::HashMap;
// Use these to configure the diff match patch engine
// I am having to just do trial and error until they maximize test passes

// Use this for maximum allowed change on finding a patch
static MATCH_THRESHOLD: f32 = 0.22;//0.22;
// Use this for maximum allowed change on either side of a deleted set of chars
static DELETE_THRESHOLD: f32 = 0.0;
// Maximum patch length allowed
static MATCH_DIST: i32 = 500;
// Maximum size of a single patch
static MATCH_BITS: i32 = 16;

// Generate a diff match patch engine with our custom settings
fn create_dmp_preconfig() -> Dmp {
    let mut dmp = Dmp::new();
    //dmp.match_distance = MATCH_DIST;
    dmp.match_threshold = MATCH_THRESHOLD;
    //dmp.patch_delete_threshold = DELETE_THRESHOLD;
    dmp.match_maxbits = MATCH_BITS;

    dmp
}

// True means keep, false means don't
fn preprocess_text(input_text: &str, comments: bool, empty_lines: bool, trailing: bool, leading: bool) -> String {
    let mut output = String::new();
    for line in input_text.split("\r\n") {
        let mut new_line = String::new();
        if !comments {
            if let Some(s) = line.splitn(2,'#').nth(1) {
                new_line = s.to_owned();
            }
        }
        if !empty_lines {
        }

        if !new_line.is_empty() || empty_lines {
            output.push_str("\r\n");
            output.push_str(&new_line);
        }
    }

    output
}

fn diff_linemode_nway(base_text: &str, modified_text: &[String]) -> (HashMap<char,String>,Vec<Vec<Diff>>,String) {
    let mut all_strings = modified_text.to_vec();
    let mut encoded_strings = Vec::new();
    all_strings.insert(0,base_text.to_owned());
    
    let mut char_to_line = HashMap::new();
    let mut line_to_char: HashMap<String,char> = HashMap::new();
    let mut current_idx: u32 = 128;
    let mut diffs = Vec::new();

    for text in all_strings {
        let real_text = text;//preprocess_text(&text,true,true,true,true);
        
        let mut encoded_text = String::new();
        for line in real_text.split("\r\n") {
        if !line_to_char.contains_key(line) {
            current_idx+=1;
            while std::char::from_u32(current_idx).is_none() {
                current_idx+=1;
            }
            line_to_char.insert(line.to_owned(), std::char::from_u32(current_idx).unwrap());
        }
        if let Some(c) = line_to_char.get(line) {
            encoded_text.push(*c);
        } else {
            eprintln!{"Something has gone horribly wrong in the n-way diff."};
        }
    }
        encoded_strings.push(encoded_text);
    }

    let mut dmp = create_dmp_preconfig();
    for text_2 in encoded_strings.iter().skip(1) {
        let mut diff = dmp.diff_main(&encoded_strings[0], text_2, false);
        dmp.diff_cleanup_efficiency(&mut diff);
        diffs.push(diff);
    }

    for (key,val) in line_to_char {
        char_to_line.insert(val, key);
    }

    (char_to_line,diffs,encoded_strings[0].clone())
}

fn patch_nway(source_text: &str, diffs: &mut Vec<Vec<Diff>>) -> Option<String> {
    let mut dmp = create_dmp_preconfig();
    let mut result_text = source_text.to_owned();
    for diff in diffs {
        let mut patch = dmp.patch_make4(source_text, diff);
        let (changed_text,applied_patches) = dmp.patch_apply(&mut patch, &result_text);
        if !applied_patches.iter().fold(true, |a,b| a & b) {
            eprintln!("Patch FAILED");
            return None;
        }
        result_text = changed_text.iter().collect();
    }
    Some(result_text)
}

pub fn diff_single_conflict(base_text: &str, modded_texts: &[String], verbose: bool) -> Option<String> {
    let (character_map,mut diffs,encoded_base) = diff_linemode_nway(base_text, modded_texts);
    //eprintln!("Created nway diff result");
    let encoded_patched = patch_nway(&encoded_base, &mut diffs)?;
    // Now decode
    //eprintln!("Created encoded patch");
    let mut result_text = String::new();
    let mut iter = 0;
    let mut max = encoded_patched.len();
    for character in encoded_patched.chars() {
        //eprintln!("searching for something in the character --{}--",character);
        let line = character_map.get(&character)?;
        result_text.push_str(line);
        result_text.push_str("\r\n");
    }
    let _a = result_text.pop();
    let _a = result_text.pop();
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
        
        assert_eq!(diff_single_conflict(&source, &[change_tier,change_cash], true),Some(both_changes));
    }
    
    #[test]
    fn test_diff_two_removals() {
        let source = "OR = \r\n{\r\n\ttier = KING\r\n\tcash = 240\r\n\treligion = rustacean\r\n}\r\n".to_owned();
        let no_tier = "OR = \r\n{\r\n\tcash = 240\r\n\treligion = rustacean\r\n}\r\n".to_owned();
        let no_cash = "OR = \r\n{\r\n\ttier = KING\r\n\treligion = rustacean\r\n}\r\n".to_owned();
        
        let both_changes = "OR = \r\n{\r\n\treligion = rustacean\r\n}\r\n".to_owned();
        
        assert_eq!(diff_single_conflict(&source, &[no_tier,no_cash], true),Some(both_changes));
    }
    
    #[test]
    fn test_diff_remove_first() {
        let source = "OR = \r\n{\r\n\ttier = KING\r\n\tcash = 240\r\n\treligion = rustacean\r\n}\r\n".to_owned();
        let no_tier = "OR = \r\n{\r\n\tcash = 240\r\n\treligion = rustacean\r\n}\r\n".to_owned();
        let change_cash = "OR = \r\n{\r\n\ttier = KING\r\n\t AND = {\r\n\tbob = jim\r\n\t zoop = zorp}\r\n\treligion = rustacean\r\n}\r\n".to_owned();
        
        let both_changes = "OR = \r\n{\r\n\t AND = {\r\n\tbob = jim\r\n\t zoop = zorp}\r\n\treligion = rustacean\r\n}\r\n".to_owned();
        
        assert_eq!(diff_single_conflict(&source, &[no_tier,change_cash], true),Some(both_changes));
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

        assert_eq!(diff_single_conflict(source, &[big_change.clone(),small_change_later.clone()], true),Some(result.clone()));
        assert_eq!(diff_single_conflict(source, &[small_change_later,big_change], true),Some(result));
    }

    #[test]
    fn test_adding_many_lines_later_remove() {
        let source = "this is some\r\ntext that will have many lines added in the middle with\r\na lot of content\r\nalso this text goes away\r\nthis doesn't go away";
        let big_change = "this is some\r\nAAAAAAAAAAAAAAAAASJHljshdflshjdfhsdjfhjsdhfljshflsdfhsd\r\na lot of\r\nsdkjfhskjdhfkjshdfkjnsdjfhkjsdhfuhsdfhljsdfljshdfjsdfhljsdhfljsdhfysdofh      \t   \r\ntext that will have many lines added in the middle with\r\na lot of content\r\nalso this text goes away\r\nthis doesn't go away".to_owned();
        let small_change_later = "this is some\r\ntext that will have many lines added in the middle with\r\na lot of changes in the middle that makes a lot of gibberish the content\r\nthis doesn't go away".to_owned();

        let result = "this is some\r\nAAAAAAAAAAAAAAAAASJHljshdflshjdfhsdjfhjsdhfljshflsdfhsd\r\na lot of\r\nsdkjfhskjdhfkjshdfkjnsdjfhkjsdhfuhsdfhljsdfljshdfjsdfhljsdhfljsdhfysdofh      \t   \r\ntext that will have many lines added in the middle with\r\na lot of changes in the middle that makes a lot of gibberish the content\r\nthis doesn't go away".to_owned();

        assert_eq!(diff_single_conflict(source, &[big_change.clone(),small_change_later.clone()], true),Some(result.clone()));
        assert_eq!(diff_single_conflict(source, &[small_change_later,big_change], true),Some(result));
    }

    #[test]
    fn test_one_changes_one_removes_same_line() {
        let source = "This is my original text\r\n\r\nthis is a line is unbalanced\r\nluckily the changed text is better.";
        let change_line = "This is my original text\r\n\r\nthis is a line is super duper balanced\r\nluckily the changed text is better.".to_owned();
        let remove_line = "This is my original text\r\n\r\nluckily the changed text is better.".to_owned();

        assert_eq!(diff_single_conflict(source, &[change_line,remove_line], false),None,"If one file tries to remove a line and another wants to change it, these are not compatible changes");
    }
}