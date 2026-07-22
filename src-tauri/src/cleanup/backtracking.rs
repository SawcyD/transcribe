use regex::Regex;

/// Which deterministic cleanup passes to run. Each maps to a user-facing toggle
/// on the Dictation page.
#[derive(Debug, Clone, Copy)]
pub struct BacktrackingOptions {
    /// Spoken corrections: "delete the last sentence", "replace X with Y".
    pub backtracking: bool,
    pub remove_filler_words: bool,
    pub remove_false_starts: bool,
}

impl Default for BacktrackingOptions {
    fn default() -> Self {
        Self {
            backtracking: true,
            remove_filler_words: true,
            remove_false_starts: true,
        }
    }
}

pub fn resolve_explicit_backtracking(input: &str) -> String {
    resolve_with_options(input, BacktrackingOptions::default())
}

pub fn resolve_with_options(input: &str, options: BacktrackingOptions) -> String {
    let mut text = input.to_string();
    if options.backtracking {
        text = remove_delete_last_sentence(&text);
        text = apply_replace_command(&text);
        text = resolve_correction_marker(&text);
        text = apply_identifier_casing(&text);
    }
    if options.remove_filler_words {
        text = remove_fillers(&text);
    }
    if options.remove_false_starts {
        text = remove_repeated_phrases(&text);
    }
    clean_spacing(&text)
}

fn remove_delete_last_sentence(input: &str) -> String {
    let marker =
        Regex::new(r"(?i)\bdelete the last sentence\b[.,;:— -]*").expect("static regex is valid");
    let Some(found) = marker.find(input) else {
        return input.to_string();
    };
    let before = input[..found.start()].trim_end();
    let after = input[found.end()..].trim_start();
    let without_final_punctuation = before
        .trim_end()
        .trim_end_matches(['.', '!', '?'])
        .trim_end();
    let sentence_start = without_final_punctuation
        .char_indices()
        .rev()
        .find(|(_, value)| matches!(value, '.' | '!' | '?'))
        .map(|(index, _)| index + 1)
        .unwrap_or(0);
    format!("{} {}", before[..sentence_start].trim(), after)
        .trim()
        .to_string()
}

fn apply_replace_command(input: &str) -> String {
    let command = Regex::new(
        r"(?i)(?:,|\.|;)?\s*\b(?:replace|change)\s+(.+?)\s+(?:with|to)\s+([^.!?]+)[.!?]?$",
    )
    .expect("static regex is valid");
    let Some(captures) = command.captures(input) else {
        return input.to_string();
    };
    let Some(full) = captures.get(0) else {
        return input.to_string();
    };
    let old = captures
        .get(1)
        .map(|value| value.as_str().trim())
        .unwrap_or_default();
    let new = captures
        .get(2)
        .map(|value| value.as_str().trim())
        .unwrap_or_default();
    let prefix = input[..full.start()].trim_end_matches([',', ' ', '.', ';']);
    let occurrence = RegexBuilderExt::literal(old);
    if occurrence.is_match(prefix) {
        format!("{}.", occurrence.replace_all(prefix, new))
            .trim()
            .to_string()
    } else {
        format!("{} {}", prefix, new).trim().to_string()
    }
}

fn resolve_correction_marker(input: &str) -> String {
    let marker = Regex::new(
        r"(?i)\s*(?:—|-|,)\s*(?:scratch that|actually|i mean|no,?\s*change that)\s*(?:—|-|,)?\s*",
    )
    .expect("static regex is valid");
    let Some(found) = marker.find(input) else {
        return input.to_string();
    };
    let before = input[..found.start()].trim();
    let after = input[found.end()..]
        .trim()
        .trim_start_matches("call it ")
        .trim_start_matches("use ");
    let lower = before.to_lowercase();
    if let Some(index) = lower.rfind(" called ") {
        return format!(
            "{} called {}",
            &before[..index],
            to_case(after.trim_end_matches('.'), "camel case")
        );
    }
    if let Some(index) = lower.rfind(" the ") {
        if after.to_lowercase().starts_with("the ") {
            return format!("{} {}", &before[..index], after);
        }
    }
    after.to_string()
}

fn apply_identifier_casing(input: &str) -> String {
    let casing = Regex::new(r"(?i)\b([a-z0-9]+(?:[ -]+[a-z0-9]+)+?)\s+(screaming snake case|camel case|pascal case|snake case|kebab case)\b").expect("static regex is valid");
    casing
        .replace_all(input, |captures: &regex::Captures<'_>| {
            let phrase = &captures[1];
            let style = captures[2].to_lowercase();
            let Some((first, remainder)) = phrase.split_once([' ', '-']) else {
                return to_case(phrase, &style);
            };
            if matches!(
                first.to_ascii_lowercase().as_str(),
                "use" | "set" | "create" | "call" | "name" | "declare" | "rename"
            ) {
                format!("{first} {}", to_case(remainder, &style))
            } else {
                to_case(phrase, &style)
            }
        })
        .into_owned()
}

fn to_case(input: &str, style: &str) -> String {
    let words = input
        .split(|character: char| !character.is_alphanumeric())
        .filter(|value| !value.is_empty())
        .map(str::to_lowercase)
        .collect::<Vec<_>>();
    match style {
        "pascal case" => words.iter().map(|word| capitalize(word)).collect(),
        "snake case" => words.join("_"),
        "screaming snake case" => words.join("_").to_uppercase(),
        "kebab case" => words.join("-"),
        _ => words
            .iter()
            .enumerate()
            .map(|(index, word)| {
                if index == 0 {
                    word.clone()
                } else {
                    capitalize(word)
                }
            })
            .collect(),
    }
}

fn capitalize(value: &str) -> String {
    let mut chars = value.chars();
    chars
        .next()
        .map(|first| first.to_uppercase().collect::<String>() + chars.as_str())
        .unwrap_or_default()
}

fn remove_fillers(input: &str) -> String {
    let fillers =
        Regex::new(r"(?i)(^|[\s,])(um+|uh+|you know|basically|kind of|sort of)($|[\s,.])")
            .expect("static regex is valid");
    fillers.replace_all(input, "$1$3").into_owned()
}

fn remove_repeated_phrases(input: &str) -> String {
    let mut words = input
        .split_whitespace()
        .map(str::to_string)
        .collect::<Vec<_>>();
    let mut changed = true;
    while changed {
        changed = false;
        'search: for size in (1..=4).rev() {
            if words.len() < size * 2 {
                continue;
            }
            for index in 0..=words.len() - size * 2 {
                let same = (0..size).all(|offset| {
                    normalize_token(&words[index + offset])
                        == normalize_token(&words[index + size + offset])
                });
                if same {
                    words.drain(index..index + size);
                    changed = true;
                    break 'search;
                }
            }
        }
    }
    words.join(" ")
}

fn normalize_token(value: &str) -> String {
    value
        .trim_matches(|character: char| !character.is_alphanumeric() && character != '_')
        .to_ascii_lowercase()
}

fn clean_spacing(input: &str) -> String {
    let spaces = Regex::new(r"[ \t]{2,}").expect("static regex is valid");
    let punctuation = Regex::new(r"\s+([,.!?;:])").expect("static regex is valid");
    let collapsed = spaces.replace_all(input.trim(), " ");
    let normalized = punctuation.replace_all(&collapsed, "$1");
    normalized.trim_start_matches([',', ';']).trim().to_string()
}

struct RegexBuilderExt;
impl RegexBuilderExt {
    fn literal(value: &str) -> Regex {
        regex::RegexBuilder::new(&regex::escape(value))
            .case_insensitive(true)
            .build()
            .expect("escaped literal is valid")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deletes_the_previous_sentence() {
        assert_eq!(
            resolve_explicit_backtracking(
                "Send it tomorrow. Delete the last sentence. Ask when they are available."
            ),
            "Ask when they are available."
        );
    }

    #[test]
    fn replaces_an_explicit_phrase() {
        assert_eq!(
            resolve_explicit_backtracking(
                "The variable is player data, change player data to profile data."
            ),
            "The variable is profile data."
        );
    }

    #[test]
    fn resolves_a_named_function_correction() {
        assert_eq!(
            resolve_explicit_backtracking(
                "Create a function called load data—actually call it load profile."
            ),
            "Create a function called loadProfile"
        );
    }

    #[test]
    fn converts_spoken_identifier_casing() {
        assert_eq!(
            resolve_explicit_backtracking("Use inventory controller Pascal case."),
            "Use InventoryController."
        );
        assert_eq!(
            resolve_explicit_backtracking("Set max retries screaming snake case."),
            "Set MAX_RETRIES."
        );
    }

    #[test]
    fn removes_repeated_false_starts_and_safe_fillers() {
        assert_eq!(
            resolve_explicit_backtracking("Um, the meeting the meeting is basically tomorrow."),
            "the meeting is tomorrow."
        );
    }
}
