use {
    serde::{Deserialize, Serialize},
    std::path::PathBuf,
};

#[derive(Serialize, Deserialize, Debug)]
pub struct Topic {
    pub name: String,
    pub desc: String,
    pub entries: Vec<Entry>,
    /// Each topic remembers what task was last selected
    pub task_sel: Option<usize>,
    /// Child topics, if any
    #[serde(default)]
    pub children: Vec<Topic>,
}

impl crate::tree::Node for Topic {
    fn children_mut(&mut self) -> &mut Vec<Self> {
        &mut self.children
    }
}

#[derive(Serialize, Deserialize, Default, Debug)]
pub struct Entry {
    pub title: String,
    pub desc: String,
    /// Whether this task is done. Only applicable if the entry kind is `Task`.
    #[serde(default)]
    pub done: bool,
    #[serde(default)]
    pub attachments: Vec<Attachment>,
    #[serde(default)]
    pub kind: EntryKind,
}

#[derive(Serialize, Deserialize, Default, Debug, PartialEq)]
pub enum EntryKind {
    /// Toggleable checkmark
    #[default]
    Task,
    /// A simple piece of information
    Info,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Attachment {
    pub filename: PathBuf,
    pub data: Vec<u8>,
}
