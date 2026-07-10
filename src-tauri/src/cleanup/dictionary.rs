use regex::RegexBuilder;

use crate::{errors::AppError, models::DictionaryEntry};

pub fn apply_dictionary(text: &str, entries: &[DictionaryEntry]) -> Result<String, AppError> {
    let mut result = text.to_string();
    let mut ordered = entries
        .iter()
        .filter(|entry| entry.enabled)
        .collect::<Vec<_>>();
    ordered.sort_by_key(|entry| std::cmp::Reverse(entry.priority));
    for entry in ordered {
        let written = entry.replacement.as_deref().unwrap_or(&entry.display_term);
        for spoken in &entry.spoken_forms {
            if spoken.trim().is_empty() {
                continue;
            }
            let escaped = regex::escape(spoken.trim());
            let pattern = if entry.whole_word_only {
                format!(r"(?P<left>^|[^\p{{L}}\p{{N}}_]){escaped}(?P<right>$|[^\p{{L}}\p{{N}}_])")
            } else {
                escaped
            };
            let regex = RegexBuilder::new(&pattern)
                .case_insensitive(!entry.case_sensitive)
                .build()
                .map_err(|error| {
                    AppError::Configuration(format!("dictionary pattern is invalid: {error}"))
                })?;
            if entry.whole_word_only {
                result = regex
                    .replace_all(&result, |captures: &regex::Captures<'_>| {
                        format!("{}{}{}", &captures["left"], written, &captures["right"])
                    })
                    .into_owned();
            } else {
                result = regex.replace_all(&result, written).into_owned();
            }
        }
    }
    Ok(collapse_spaces(&result))
}

fn collapse_spaces(value: &str) -> String {
    let horizontal = regex::Regex::new(r"[\t ]{2,}").expect("static regex is valid");
    horizontal.replace_all(value.trim(), " ").into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::DictionaryCategory;
    use chrono::Utc;

    fn entry(spoken: &str, written: &str) -> DictionaryEntry {
        DictionaryEntry {
            id: "one".into(),
            display_term: written.into(),
            spoken_forms: vec![spoken.into()],
            replacement: Some(written.into()),
            category: DictionaryCategory::Replacement,
            priority: 10,
            case_sensitive: false,
            whole_word_only: true,
            enabled: true,
            usage_count: 0,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    #[test]
    fn replaces_whole_spoken_forms() {
        assert_eq!(
            apply_dictionary(
                "Build this with Terry version two.",
                &[entry("Terry version two", "Tauri v2")]
            )
            .unwrap(),
            "Build this with Tauri v2."
        );
    }

    #[test]
    fn does_not_replace_inside_identifiers() {
        assert_eq!(
            apply_dictionary("myroblox tsvalue", &[entry("roblox ts", "roblox-ts")]).unwrap(),
            "myroblox tsvalue"
        );
    }
}
