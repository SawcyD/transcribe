use chrono::Utc;
use rusqlite::params;
use serde::Serialize;

use crate::{
    assistant_memory::{self, RecalledTurn},
    errors::AppError,
};

use super::Database;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AssistantChatSummary {
    pub id: String,
    pub title: String,
    pub application_name: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub turn_count: i64,
    pub preview: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AssistantChatTurn {
    pub question: String,
    pub answer: String,
    pub created_at: String,
}

impl Database {
    /// Appends a question/answer to a chat, creating the chat on first use.
    pub fn append_assistant_turn(
        &self,
        chat_id: &str,
        application_name: Option<&str>,
        question: &str,
        answer: &str,
    ) -> Result<(), AppError> {
        let now = Utc::now().to_rfc3339();
        self.with_connection(|connection| {
            connection.execute(
                "INSERT INTO assistant_chats(id, title, application_name, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?4)
                 ON CONFLICT(id) DO UPDATE SET updated_at = excluded.updated_at",
                params![chat_id, assistant_memory::title_from(question), application_name, now],
            )?;
            connection.execute(
                "INSERT INTO assistant_turns(chat_id, question, answer, created_at) VALUES (?1, ?2, ?3, ?4)",
                params![chat_id, question, answer, now],
            )?;
            Ok(())
        })
    }

    pub fn list_assistant_chats(&self) -> Result<Vec<AssistantChatSummary>, AppError> {
        self.with_connection(|connection| {
            let mut statement = connection.prepare(
                "SELECT c.id, c.title, c.application_name, c.created_at, c.updated_at,
                        (SELECT COUNT(*) FROM assistant_turns t WHERE t.chat_id = c.id),
                        COALESCE((SELECT t.answer FROM assistant_turns t WHERE t.chat_id = c.id ORDER BY t.id DESC LIMIT 1), '')
                 FROM assistant_chats c ORDER BY c.updated_at DESC LIMIT 500",
            )?;
            let rows = statement
                .query_map([], |row| {
                    let preview: String = row.get(6)?;
                    Ok(AssistantChatSummary {
                        id: row.get(0)?,
                        title: row.get(1)?,
                        application_name: row.get(2)?,
                        created_at: row.get(3)?,
                        updated_at: row.get(4)?,
                        turn_count: row.get(5)?,
                        preview: preview.chars().take(160).collect(),
                    })
                })?
                .collect::<Result<Vec<_>, _>>()?;
            Ok(rows)
        })
    }

    pub fn get_assistant_chat(&self, chat_id: &str) -> Result<Vec<AssistantChatTurn>, AppError> {
        self.with_connection(|connection| {
            let mut statement = connection.prepare(
                "SELECT question, answer, created_at FROM assistant_turns WHERE chat_id = ?1 ORDER BY id",
            )?;
            let rows = statement
                .query_map(params![chat_id], |row| {
                    Ok(AssistantChatTurn {
                        question: row.get(0)?,
                        answer: row.get(1)?,
                        created_at: row.get(2)?,
                    })
                })?
                .collect::<Result<Vec<_>, _>>()?;
            Ok(rows)
        })
    }

    pub fn delete_assistant_chat(&self, chat_id: &str) -> Result<(), AppError> {
        self.with_connection(|connection| {
            connection.execute("DELETE FROM assistant_turns WHERE chat_id = ?1", params![chat_id])?;
            connection.execute("DELETE FROM assistant_chats WHERE id = ?1", params![chat_id])?;
            Ok(())
        })
    }

    /// The saved turns from *other* chats that best match the question.
    pub fn recall_assistant_turns(
        &self,
        question: &str,
        exclude_chat: Option<&str>,
        limit: usize,
    ) -> Result<Vec<RecalledTurn>, AppError> {
        let keywords = assistant_memory::keywords(question);
        if keywords.is_empty() {
            return Ok(Vec::new());
        }
        // Coarse SQL prefilter on any keyword, then rank in Rust.
        let clauses = keywords
            .iter()
            .map(|_| "(LOWER(t.question) LIKE ? OR LOWER(t.answer) LIKE ?)")
            .collect::<Vec<_>>()
            .join(" OR ");
        let sql = format!(
            "SELECT c.title, t.created_at, t.question, t.answer, t.chat_id
             FROM assistant_turns t JOIN assistant_chats c ON c.id = t.chat_id
             WHERE {clauses} ORDER BY t.id DESC LIMIT 200"
        );
        let patterns: Vec<String> = keywords
            .iter()
            .flat_map(|keyword| {
                let pattern = format!("%{keyword}%");
                [pattern.clone(), pattern]
            })
            .collect();
        self.with_connection(|connection| {
            let mut statement = connection.prepare(&sql)?;
            let mut scored = statement
                .query_map(rusqlite::params_from_iter(patterns.iter()), |row| {
                    Ok((
                        RecalledTurn {
                            chat_title: row.get(0)?,
                            created_at: row.get(1)?,
                            question: row.get(2)?,
                            answer: row.get(3)?,
                        },
                        row.get::<_, String>(4)?,
                    ))
                })?
                .filter_map(Result::ok)
                .filter(|(_, chat)| Some(chat.as_str()) != exclude_chat)
                .map(|(turn, _)| {
                    let score = assistant_memory::score(&keywords, &turn.question, &turn.answer);
                    (score, turn)
                })
                .filter(|(score, _)| *score >= 2)
                .collect::<Vec<_>>();
            // Best match first; newer wins ties (rows arrive newest-first).
            scored.sort_by(|left, right| right.0.cmp(&left.0));
            Ok(scored.into_iter().take(limit).map(|(_, turn)| turn).collect())
        })
    }
}
