use anyhow::{Context, Result};
use chrono::{NaiveDate, Utc};
use dialoguer::{
    theme::ColorfulTheme,
    {Input, Select},
};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::Path,
};

#[derive(Debug, Serialize, Deserialize)]
struct Task {
    id: usize,
    description: String,
    completed: bool,
    tags: Vec<String>,
    due_date: Option<NaiveDate>,
}

#[derive(Debug, Serialize, Deserialize)]
struct TaskManager {
    tasks: Vec<Task>,
    next_id: usize,
}

impl TaskManager {
    fn new() -> Self {
        TaskManager {
            tasks: Vec::new(),
            next_id: 1,
        }
    }

    fn add_task(&mut self, task: Task) {
        self.tasks.push(task);
    }

    fn complete_task(&mut self, id: usize) -> Option<&Task> {
        if let Some(task) = self.tasks.iter_mut().find(|t| t.id == id) {
            task.completed = true;
            Some(task)
        } else {
            None
        }
    }

    fn delete_task(&mut self, id: usize) -> Option<Task> {
        if let Some(pos) = self.tasks.iter().position(|t| t.id == id) {
            Some(self.tasks.remove(pos))
        } else {
            None
        }
    }

    fn edit_task(&mut self, id: usize) -> Option<&Task> {
        if let Some(task) = self.tasks.iter_mut().find(|t| t.id == id) {
            let description: String = Input::with_theme(&ColorfulTheme::default())
                .with_prompt("New description")
                .default(task.description.clone())
                .interact()
                .unwrap();

            let due_date = Input::with_theme(&ColorfulTheme::default())
                .with_prompt("Due date (YYYY-MM-DD) (leave empty to remove)")
                .allow_empty(true)
                .validate_with(|input: &String| {
                    if input.is_empty() {
                        return Ok(());
                    }
                    NaiveDate::parse_from_str(input, "%Y-%m-%d")
                        .map(|_| ())
                        .map_err(|_| "Invalid date format. Use YYYY-MM-DD".into())
                })
                .interact()
                .unwrap();

            let tags = Input::with_theme(&ColorfulTheme::default())
                .with_prompt("Tags (comma-separated)")
                .default(task.tags.join(", "))
                .interact()
                .map(|s: String| {
                    s.split(',')
                        .map(|tag| tag.trim().to_lowercase())
                        .filter(|tag| !tag.is_empty())
                        .collect()
                })
                .unwrap();

            task.description = description;
            task.due_date = due_date
                .parse::<NaiveDate>()
                .ok();
            task.tags = tags;
            Some(task)
        } else {
            None
        }
    }

    fn save(&self, path: &Path) -> Result<()> {
        let contents = serde_json::to_string_pretty(self)?;
        fs::write(path, contents)?;
        Ok(())
    }

    fn load(path: &Path) -> Result<Self> {
        if path.exists() {
            let contents = fs::read_to_string(path)?;
            let manager = serde_json::from_str(&contents)?;
            Ok(manager)
        } else {
            Ok(TaskManager::new())
        }
    }

    fn list_tasks(&self, filter: Option<&str>) {
        let today = Utc::now().naive_utc().date();

        println!("\n{}", "Tasks:".bold().underline());
        for task in &self.tasks {
            if let Some(filter) = filter {
                if !task.tags.contains(&filter.to_lowercase()) && 
                   !task.description.to_lowercase().contains(&filter.to_lowercase()) {
                    continue;
                }
            }

            let status = if task.completed {
                "[✓]".green()
            } else {
                match task.due_date {
                    Some(due) if due < today => "[!]".red(),
                    Some(due) if due == today => "[!]".yellow(),
                    _ => "[ ]".normal(),
                }
            };

            let mut parts = vec![
                status,
                format!("#{}", task.id).cyan().normal(),
                task.description.as_str().normal(),
            ];

            if let Some(due_date) = task.due_date {
                let due_str = format!("({})", due_date.format("%Y-%m-%d"));
                let due_display = if due_date < today {
                    due_str.red()
                } else if due_date == today {
                    due_str.yellow()
                } else {
                    due_str.normal()
                };
                parts.push(due_display);
            }

            if !task.tags.is_empty() {
                parts.push(format!("[{}]", task.tags.join(", ")).blue().normal());
            }

            println!("{}", parts.join(" "));
        }
        println!();
    }
}

fn main() -> Result<()> {
    let data_dir = dirs::data_dir()
        .context("Could not find data directory")?
        .join("rust_task_manager");
    
    if !data_dir.exists() {
        fs::create_dir_all(&data_dir)?;
    }

    let data_file = data_dir.join("tasks.json");
    let mut manager = TaskManager::load(&data_file)?;

    loop {
        let choices = vec![
            "Add Task",
            "List Tasks",
            "Complete Task",
            "Delete Task",
            "Edit Task",
            "Search Tasks",
            "Exit",
        ];

        let selection = Select::with_theme(&ColorfulTheme::default())
            .with_prompt("What would you like to do?")
            .items(&choices)
            .default(0)
            .interact()?;

        match selection {
            0 => {
                let description: String = Input::with_theme(&ColorfulTheme::default())
                    .with_prompt("Task description")
                    .interact()?;

                let due_date: String = Input::with_theme(&ColorfulTheme::default())
                    .with_prompt("Due date (YYYY-MM-DD) (optional)")
                    .allow_empty(true)
                    .validate_with(|input: &String| {
                        if input.is_empty() {
                            return Ok(());
                        }
                        NaiveDate::parse_from_str(input, "%Y-%m-%d")
                            .map(|_| ())
                            .map_err(|_| "Invalid date format. Use YYYY-MM-DD".into())
                    })
                    .interact()?;

                let tags: String = Input::with_theme(&ColorfulTheme::default())
                    .with_prompt("Tags (comma-separated, optional)")
                    .allow_empty(true)
                    .interact()?;

                let task = Task {
                    id: manager.next_id,
                    description,
                    completed: false,
                    tags: tags
                        .split(',')
                        .map(|s| s.trim().to_lowercase())
                        .filter(|s| !s.is_empty())
                        .collect(),
                    due_date: due_date.parse().ok(),
                };

                manager.next_id += 1;
                manager.add_task(task);
            }
            1 => manager.list_tasks(None),
            2 => {
                let task_id: usize = Input::with_theme(&ColorfulTheme::default())
                    .with_prompt("Task ID to complete")
                    .interact()?;

                if let Some(task) = manager.complete_task(task_id) {
                    println!("Completed task #{}: {}", task.id, task.description);
                } else {
                    println!("{}", "Task not found!".red());
                }
            }
            3 => {
                let task_id: usize = Input::with_theme(&ColorfulTheme::default())
                    .with_prompt("Task ID to delete")
                    .interact()?;

                if let Some(task) = manager.delete_task(task_id) {
                    println!("Deleted task #{}: {}", task.id, task.description);
                } else {
                    println!("{}", "Task not found!".red());
                }
            }
            4 => {
                let task_id: usize = Input::with_theme(&ColorfulTheme::default())
                    .with_prompt("Task ID to edit")
                    .interact()?;

                if let Some(task) = manager.edit_task(task_id) {
                    println!("Updated task #{}", task.id);
                } else {
                    println!("{}", "Task not found!".red());
                }
            }
            5 => {
                let filter: String = Input::with_theme(&ColorfulTheme::default())
                    .with_prompt("Search (tag or text)")
                    .interact()?;

                manager.list_tasks(Some(&filter));
            }
            6 => break,
            _ => unreachable!(),
        }

        manager.save(&data_file)?;
    }

    Ok(())
}
