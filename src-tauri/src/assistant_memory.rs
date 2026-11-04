//! Lightweight long-term memory for the assistant: when you ask something,
//! earlier saved turns that share meaningful words with the question are
//! recalled and given to the model as background ("remember that interview I
//! mentioned?"). Pure functions here; storage lives in `database`.

use serde::Serialize;

/// One saved question/answer pair, as recalled from history.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecalledTurn {
    pub chat_title: String,
    pub created_at: String,
    pub question: String,
    pub answer: String,
}

const STOPWORDS: &[&str] = &[
    "about", "after", "again", "also", "because", "been", "before", "being", "could", "does",
    "doing", "from", "have", "here", "into", "just", "know", "like", "made", "make", "more",
    "much", "remember", "said", "should", "some", "tell", "than", "that", "their", "them",
    "then", "there", "these", "they", "thing", "this", "time", "talking", "talked", "very",
    "want", "were", "what", "when", "where", "which", "while", "with", "would", "your", "you're",
    "explain", "screen", "mean", "means", "help", "please", "okay",
];

/// Distinct, lowercase words worth searching on (4+ letters, not filler).
pub fn keywords(text: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for word in text
        .split(|c: char| !c.is_alphanumeric() && c != '\'')
        .map(|word| word.trim_matches('\'').to_lowercase())
    {
        if word.chars().count() >= 4 && !STOPWORDS.contains(&word.as_str()) && !out.contains(&word) {
            out.push(word);
        }
    }
    out.truncate(8);
    out
}

/// How many distinct keywords appear in a turn. Questions count double: what
/// the user said is a better memory cue than the model's wording.
pub fn score(keywords: &[String], question: &str, answer: &str) -> usize {
    let question = question.to_lowercase();
    let answer = answer.to_lowercase();
    keywords
        .iter()
        .map(|keyword| {
            if question.contains(keyword.as_str()) {
                2
            } else if answer.contains(keyword.as_str()) {
                1
            } else {
                0
            }
        })
        .sum()
}

fn clip(text: &str, max: usize) -> String {
    let text = text.trim();
    if text.chars().count() <= max {
        return text.to_string();
    }
    format!("{}…", text.chars().take(max).collect::<String>().trim_end())
}

/// The system-prompt addendum describing recalled memories, or None.
pub fn prompt_section(recalled: &[RecalledTurn]) -> Option<String> {
    if recalled.is_empty() {
        return None;
    }
    let mut section = String::from(
        "Earlier conversations with this user that may be relevant. Use them only if they help, and say so when you're drawing on them (\"Last time you mentioned…\"):\n",
    );
    for turn in recalled {
        let day = turn.created_at.get(..10).unwrap_or(&turn.created_at);
        section.push_str(&format!(
            "- [{day}, \"{}\"] User: {} | You: {}\n",
            clip(&turn.chat_title, 60),
            clip(&turn.question, 300),
            clip(&turn.answer, 400)
        ));
    }
    Some(section)
}

/// A chat title from its first question.
pub fn title_from(question: &str) -> String {
    let title = clip(question.lines().next().unwrap_or(question), 60);
    if title.is_empty() {
        "Untitled chat".into()
    } else {
        title
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keywords_skip_filler_and_duplicates() {
        assert_eq!(
            keywords("Remember the time when I was talking about that interview? The INTERVIEW at Stripe."),
            vec!["interview", "stripe"]
        );
    }

    #[test]
    fn questions_outweigh_answers() {
        let words = keywords("my interview prep");
        assert!(score(&words, "How do I prep for an interview?", "") > score(&words, "", "interview prep tips"));
        assert_eq!(score(&words, "unrelated", "nothing"), 0);
    }

    #[test]
    fn prompt_section_is_dated_and_clipped() {
        let turn = RecalledTurn {
            chat_title: "Interview prep".into(),
            created_at: "2026-10-28T10:00:00Z".into(),
            question: "I have an interview at Stripe on Friday".into(),
            answer: "a".repeat(1000),
        };
        let section = prompt_section(&[turn]).unwrap();
        assert!(section.contains("[2026-10-28, \"Interview prep\"] User: I have an interview at Stripe on Friday"));
        assert!(section.len() < 1000);
        assert!(prompt_section(&[]).is_none());
    }
}
