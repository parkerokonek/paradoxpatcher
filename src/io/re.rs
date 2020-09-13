use regex::Regex;

/// Performs a search using a pre-compiled regular expression on an input string and returns all matching strings
/// If no matches are found, this returns an empty vector.
/// # Arguments
/// 
/// * `input` - Text to search over
/// 
/// * `re` - the pre-compiled regex to match against
/// 
/// * `all_matches` - if true, return all matches, otherwise only return the first match
/// 
pub fn grep(input: &str, re: &Regex, all_matches: bool) -> Vec<String> {
    let mut matches = re.find_iter(&input);
    if all_matches {
        return matches.map(|x| x.as_str().to_string()).collect();
    } else if let Some(valid) = matches.next() {
        return vec![valid.as_str().to_string()];
    }
    
    Vec::new()
}

/// Utility function to remove the quotes from both ends of a string
/// #Arguments
/// 
/// * `input` - string to trim of quotes
pub fn trim_quotes(input: &str) -> String {
    let left: Vec<&str> = input.split('"').collect();
    if left.len() == 3 {
        return left[1].to_string();
    }
    String::new()
}