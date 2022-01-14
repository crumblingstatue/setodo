use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct Topic {
    pub name: String,
    pub desc: String,
    pub tasks: Vec<Task>,
    /// Each topic remembers what task was last selected
    pub task_sel: Option<usize>,
    /// Child topics, if any
    #[serde(default)]
    pub children: Vec<Topic>,
}

#[derive(Serialize, Deserialize, Default, Debug)]
pub struct Task {
    pub title: String,
    pub desc: String,
    #[serde(default)]
    pub done: bool,
    #[serde(default)]
    pub attachments: Vec<Attachment>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Attachment {
    pub filename: PathBuf,
    pub data: Vec<u8>,
}
