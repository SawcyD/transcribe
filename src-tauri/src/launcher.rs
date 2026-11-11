//! What the voice agent can do outside the app: open apps and websites, and
//! search the web. It only supplies a name, an address or a query: it never
//! gets to run a command line.

use std::path::{Path, PathBuf};

fn collect_shortcuts(dir: &Path, found: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    for path in entries.flatten().map(|entry| entry.path()) {
        if path.is_dir() {
            collect_shortcuts(&path, found);
        } else if path.extension().is_some_and(|extension| extension.eq_ignore_ascii_case("lnk")) {
            found.push(path);
        }
    }
}

fn normalize(text: &str) -> String {
    text.chars().filter(|c| c.is_alphanumeric()).flat_map(char::to_lowercase).collect()
}

/// Picks the name that best matches what was asked for: an exact match, or
/// else the shortest name containing it ("obs" finds "OBS Studio" before
/// "OBS Studio (Safe Mode)").
pub(crate) fn best_match(names: &[String], query: &str) -> Option<usize> {
    let query = normalize(query);
    if query.is_empty() {
        return None;
    }
    names.iter().position(|name| normalize(name) == query).or_else(|| {
        names
            .iter()
            .enumerate()
            .filter(|(_, name)| normalize(name).contains(&query))
            .min_by_key(|(_, name)| name.len())
            .map(|(index, _)| index)
    })
}

/// Opens an installed app by its Start Menu name. Returns what happened, in
/// words the agent can say.
// ponytail: Start Menu shortcuts only, so Microsoft Store apps (which have no
// shortcut file) aren't found. Add the AppsFolder list if that matters.
pub fn open_app(name: &str) -> String {
    let mut shortcuts = Vec::new();
    for variable in ["APPDATA", "ProgramData"] {
        if let Some(root) = std::env::var_os(variable) {
            collect_shortcuts(&Path::new(&root).join("Microsoft/Windows/Start Menu/Programs"), &mut shortcuts);
        }
    }
    let names: Vec<String> = shortcuts
        .iter()
        .map(|path| path.file_stem().unwrap_or_default().to_string_lossy().into_owned())
        .collect();
    match best_match(&names, name) {
        Some(index) => match tauri_plugin_opener::open_path(&shortcuts[index], None::<&str>) {
            Ok(()) => format!("Opened {}.", names[index]),
            Err(error) => format!("Couldn't open {}: {error}", names[index]),
        },
        None => format!("There is no app called {name} in the Start Menu."),
    }
}

/// Opens a web address in the default browser.
pub fn open_url(url: &str) -> String {
    if !(url.starts_with("https://") || url.starts_with("http://")) {
        return "Only web addresses starting with https:// can be opened.".into();
    }
    match tauri_plugin_opener::open_url(url, None::<&str>) {
        Ok(()) => format!("Opened {url}."),
        Err(error) => format!("Couldn't open {url}: {error}"),
    }
}

/// Searches the web through Tavily and returns a short answer with sources,
/// as text the agent can read out or put on the whiteboard.
pub async fn search_web(query: &str) -> String {
    use crate::security::{self, CredentialKind};
    if query.trim().is_empty() {
        return "There was nothing to search for.".into();
    }
    let Ok(Ok(key)) = tokio::task::spawn_blocking(|| security::get(CredentialKind::Search)).await else {
        return "Web search isn't set up. The user can add a Tavily key in the Assistant settings.".into();
    };
    let body = serde_json::json!({ "query": query, "search_depth": "basic", "max_results": 5, "include_answer": true });
    let response = reqwest::Client::new()
        .post("https://api.tavily.com/search")
        .bearer_auth(key)
        .timeout(std::time::Duration::from_secs(15))
        .json(&body)
        .send()
        .await
        .and_then(|response| response.error_for_status());
    let found: serde_json::Value = match response {
        Ok(response) => response.json().await.unwrap_or_default(),
        Err(error) => return format!("The search failed: {}", error.without_url()),
    };
    let mut text = String::new();
    if let Some(answer) = found["answer"].as_str().filter(|answer| !answer.is_empty()) {
        text.push_str(&format!("Summary: {answer}\n"));
    }
    for result in found["results"].as_array().into_iter().flatten().take(5) {
        let snippet: String = result["content"].as_str().unwrap_or_default().chars().take(300).collect();
        text.push_str(&format!(
            "- {} ({}): {snippet}\n",
            result["title"].as_str().unwrap_or("Untitled"),
            result["url"].as_str().unwrap_or_default()
        ));
    }
    if text.is_empty() {
        "The search found nothing.".into()
    } else {
        text
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_the_closest_app_name() {
        let names: Vec<String> = ["OBS Studio (Safe Mode)", "OBS Studio", "Visual Studio Code", "Notepad"]
            .map(String::from)
            .to_vec();
        assert_eq!(best_match(&names, "obs"), Some(1));
        assert_eq!(best_match(&names, "visual studio code"), Some(2));
        assert_eq!(best_match(&names, "Notepad"), Some(3));
        assert_eq!(best_match(&names, "photoshop"), None);
        assert_eq!(best_match(&names, "  "), None);
        assert!(open_url("file:///C:/Windows").starts_with("Only web"));
    }
}
