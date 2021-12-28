use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct Topic {
    pub name: String,
    pub desc: String,
    pub tasks: Vec<Task>,
    /// Each topic remembers what task was last selected
    pub task_sel: Option<usize>,
}

#[derive(Serialize, Deserialize)]
pub struct Task {
    pub title: String,
    pub desc: String,
    #[serde(default)]
    pub done: bool,
}
