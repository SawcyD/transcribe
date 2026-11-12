//! The day planner: tasks by day, optionally at a time and optionally after
//! another task. Kept in `planner.json` in the app's data folder; the
//! planner window and Flow both change it through here.

use std::{path::PathBuf, sync::Mutex};

use chrono::{Local, NaiveDate, NaiveTime};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager};

use crate::errors::AppError;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Task {
    #[serde(default)]
    pub id: String,
    pub title: String,
    /// YYYY-MM-DD
    pub day: String,
    /// HH:MM (24-hour), or none for "any time that day".
    #[serde(default)]
    pub time: Option<String>,
    #[serde(default)]
    pub done: bool,
    /// The task this one comes after, for the flow chart.
    #[serde(default)]
    pub after: Option<String>,
}

/// One writer at a time, so the window and Flow can't overwrite each other.
static FILE_LOCK: Mutex<()> = Mutex::new(());

pub fn today() -> String {
    Local::now().date_naive().format("%Y-%m-%d").to_string()
}

fn path(app: &AppHandle) -> Result<PathBuf, AppError> {
    let folder = app
        .path()
        .app_data_dir()
        .map_err(|error| AppError::Configuration(error.to_string()))?;
    std::fs::create_dir_all(&folder).map_err(|error| AppError::Configuration(error.to_string()))?;
    Ok(folder.join("planner.json"))
}

fn read(app: &AppHandle) -> Result<Vec<Task>, AppError> {
    match std::fs::read_to_string(path(app)?) {
        Ok(content) => serde_json::from_str(&content).map_err(|error| AppError::Configuration(format!("planner.json is unreadable: {error}"))),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(Vec::new()),
        Err(error) => Err(AppError::Configuration(error.to_string())),
    }
}

fn write(app: &AppHandle, tasks: &[Task]) -> Result<(), AppError> {
    let file = path(app)?;
    let partial = file.with_extension("json.part");
    let content = serde_json::to_string_pretty(tasks).map_err(|error| AppError::Configuration(error.to_string()))?;
    std::fs::write(&partial, content)
        .and_then(|()| std::fs::rename(&partial, &file))
        .map_err(|error| AppError::Configuration(error.to_string()))?;
    let _ = app.emit("planner-changed", ());
    Ok(())
}

pub fn all(app: &AppHandle) -> Result<Vec<Task>, AppError> {
    let _guard = FILE_LOCK.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    read(app)
}

/// Adds a task (empty id) or replaces the one with the same id.
pub fn save(app: &AppHandle, task: Task) -> Result<Task, AppError> {
    let _guard = FILE_LOCK.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    let mut tasks = read(app)?;
    let saved = upsert(&mut tasks, task).map_err(AppError::Configuration)?;
    write(app, &tasks)?;
    Ok(saved)
}

pub fn remove(app: &AppHandle, id: &str) -> Result<(), AppError> {
    let _guard = FILE_LOCK.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    let mut tasks = read(app)?;
    if !without(&mut tasks, id) {
        return Err(AppError::Configuration("there is no task with that id".into()));
    }
    write(app, &tasks)
}

fn upsert(tasks: &mut Vec<Task>, mut task: Task) -> Result<Task, String> {
    task.title = task.title.split_whitespace().collect::<Vec<_>>().join(" ");
    if task.title.is_empty() || task.title.chars().count() > 200 {
        return Err("a task needs a title of up to 200 characters".into());
    }
    NaiveDate::parse_from_str(&task.day, "%Y-%m-%d").map_err(|_| format!("\"{}\" is not a date like 2026-11-12", task.day))?;
    task.time = task.time.filter(|time| !time.trim().is_empty());
    if let Some(time) = &task.time {
        let parsed = NaiveTime::parse_from_str(time.trim(), "%H:%M").map_err(|_| format!("\"{time}\" is not a 24-hour time like 15:30"))?;
        task.time = Some(parsed.format("%H:%M").to_string());
    }
    task.after = task.after.filter(|after| !after.is_empty());
    if let Some(after) = &task.after {
        if *after == task.id || !tasks.iter().any(|other| other.id == *after) {
            return Err("\"after\" must be the id of another task".into());
        }
    }
    match tasks.iter_mut().find(|existing| !task.id.is_empty() && existing.id == task.id) {
        Some(existing) => *existing = task.clone(),
        None => {
            if !task.id.is_empty() {
                return Err("there is no task with that id".into());
            }
            task.id = uuid::Uuid::new_v4().to_string()[..8].to_string();
            tasks.push(task.clone());
        }
    }
    Ok(task)
}

/// Removes a task, and unhooks any task that came after it.
fn without(tasks: &mut Vec<Task>, id: &str) -> bool {
    let before = tasks.len();
    tasks.retain(|task| task.id != id);
    for task in tasks.iter_mut().filter(|task| task.after.as_deref() == Some(id)) {
        task.after = None;
    }
    tasks.len() != before
}

/// A day's tasks as text for Flow: time first, then the rest.
pub fn describe_day(tasks: &[Task], day: &str) -> String {
    let mut today: Vec<&Task> = tasks.iter().filter(|task| task.day == day).collect();
    if today.is_empty() {
        return format!("Nothing is planned for {day}.");
    }
    today.sort_by(|a, b| (a.time.is_none(), &a.time).cmp(&(b.time.is_none(), &b.time)));
    let lines: Vec<String> = today
        .iter()
        .map(|task| {
            format!(
                "- [{}] {}{}{}{}",
                task.id,
                task.time.as_deref().map(|time| format!("{time} ")).unwrap_or_default(),
                task.title,
                if task.done { " (done)" } else { "" },
                task.after.as_deref().map(|after| format!(" (after {after})")).unwrap_or_default()
            )
        })
        .collect();
    format!("Plan for {day}:\n{}", lines.join("\n"))
}

/// Shows the planner window on a day.
pub fn show(app: &AppHandle, day: &str) -> Result<(), AppError> {
    let window = app
        .get_webview_window("planner")
        .ok_or_else(|| AppError::Windows("planner window is unavailable".into()))?;
    if !window.is_visible().unwrap_or(false) {
        // Bottom-right, clear of the taskbar, the first time it opens.
        if let (Ok(Some(monitor)), Ok(size)) = (window.current_monitor(), window.outer_size()) {
            let area = crate::dock_layout::MonitorArea::of(&monitor);
            let margin = (24.0 * area.scale) as i32;
            let x = area.x + area.width as i32 - size.width as i32 - margin;
            let y = area.y + area.height as i32 - size.height as i32 - (72.0 * area.scale) as i32;
            let _ = window.set_position(tauri::Position::Physical(tauri::PhysicalPosition::new(x, y)));
        }
    }
    let _ = app.emit("planner-show-day", day);
    let _ = window.show();
    let _ = window.unminimize();
    window.set_focus().map_err(|error| AppError::Windows(error.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn task(title: &str, day: &str, time: Option<&str>) -> Task {
        Task { id: String::new(), title: title.into(), day: day.into(), time: time.map(Into::into), done: false, after: None }
    }

    #[test]
    fn adds_checks_and_links_tasks() {
        let mut tasks = Vec::new();
        let outline = upsert(&mut tasks, task("  Draft   outline ", "2026-11-12", Some("9:05"))).unwrap();
        assert_eq!(outline.title, "Draft outline");
        assert_eq!(outline.time.as_deref(), Some("09:05"));
        let review = upsert(&mut tasks, Task { after: Some(outline.id.clone()), ..task("Review PR", "2026-11-12", None) }).unwrap();
        assert!(upsert(&mut tasks, task("Bad day", "tomorrow", None)).is_err());
        assert!(upsert(&mut tasks, task("Bad time", "2026-11-12", Some("3pm"))).is_err());
        assert!(upsert(&mut tasks, Task { after: Some("missing".into()), ..task("Orphan", "2026-11-12", None) }).is_err());

        // Moving a task keeps its id.
        let moved = upsert(&mut tasks, Task { day: "2026-11-13".into(), ..outline.clone() }).unwrap();
        assert_eq!(moved.id, outline.id);
        assert_eq!(tasks.len(), 2);

        // Removing a task unhooks what came after it.
        assert!(without(&mut tasks, &outline.id));
        assert_eq!(tasks[0].id, review.id);
        assert_eq!(tasks[0].after, None);
    }

    #[test]
    fn describes_a_day_timed_tasks_first() {
        let mut tasks = Vec::new();
        upsert(&mut tasks, task("Anytime thing", "2026-11-12", None)).unwrap();
        upsert(&mut tasks, task("Standup", "2026-11-12", Some("09:30"))).unwrap();
        let text = describe_day(&tasks, "2026-11-12");
        assert!(text.find("Standup").unwrap() < text.find("Anytime thing").unwrap());
        assert_eq!(describe_day(&tasks, "2026-11-13"), "Nothing is planned for 2026-11-13.");
    }
}
