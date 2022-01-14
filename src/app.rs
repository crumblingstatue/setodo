use crate::data::{Attachment, Task, Topic};
use eframe::{
    egui::{self, Button, CollapsingHeader, Key, RichText, ScrollArea, TextBuffer},
    epi,
};
use rmp_serde::Serializer;
use serde::{Deserialize, Serialize};
use std::{error::Error, fs::File, path::PathBuf};

#[derive(Default, Serialize, Deserialize)]
pub struct TodoApp {
    topic_sel: Vec<usize>,
    topics: Vec<Topic>,
    #[serde(skip)]
    temp: TodoAppTemp,
}

/// Transient data, not saved during serialization
struct TodoAppTemp {
    state: UiState,
}

impl Default for TodoAppTemp {
    fn default() -> Self {
        Self {
            state: UiState::Normal,
        }
    }
}

#[derive(PartialEq, Eq)]
enum UiState {
    Normal,
    AddTopic(String),
    AddSubtopic {
        name: String,
        parent_idx: Vec<usize>,
    },
    AddTask(String),
}

impl UiState {
    fn add_topic() -> Self {
        Self::AddTopic(String::default())
    }
    fn add_subtopic(parent_idx: Vec<usize>) -> Self {
        Self::AddSubtopic {
            name: String::default(),
            parent_idx,
        }
    }
    fn add_task() -> Self {
        Self::AddTask(String::default())
    }
}

fn file_name() -> PathBuf {
    dirs::home_dir().unwrap().join(".setodo.dat")
}

impl TodoApp {
    pub fn load() -> Result<Self, Box<dyn Error>> {
        let file = File::open(file_name())?;
        let dec = zstd::stream::read::Decoder::new(file)?;
        Ok(rmp_serde::from_read(dec)?)
    }
    fn save(&self) -> Result<(), Box<dyn Error>> {
        let file = File::create(file_name())?;
        let mut enc = zstd::stream::write::Encoder::new(file, zstd::DEFAULT_COMPRESSION_LEVEL)?;
        self.serialize(&mut Serializer::new(&mut enc))?;
        enc.finish()?;
        Ok(())
    }
}

impl epi::App for TodoApp {
    fn name(&self) -> &str {
        "Simple Egui Todo"
    }

    fn on_exit(&mut self) {
        TodoApp::save(self).unwrap();
    }

    fn update(&mut self, ctx: &egui::Context, frame: &epi::Frame) {
        if ctx.input().key_pressed(Key::Escape) {
            frame.quit();
        }
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.set_width(300.0);
                    ui.set_height(400.0);
                    ui.heading("Topics");
                    ScrollArea::vertical().show(ui, |ui| {
                        topics_ui(&self.topics, &mut Vec::new(), &mut self.topic_sel, ui);
                    });
                    ui.horizontal(|ui| match &mut self.temp.state {
                        UiState::AddTopic(name) => {
                            let clicked = ui.button("✔").clicked();
                            if ui.button("🗙").clicked() || ui.input().key_pressed(egui::Key::Escape)
                            {
                                self.temp.state = UiState::Normal;
                            } else {
                                ui.text_edit_singleline(name).request_focus();
                                if clicked || ui.input().key_pressed(egui::Key::Enter) {
                                    self.topics.push(Topic {
                                        name: name.take(),
                                        desc: String::new(),
                                        tasks: Vec::new(),
                                        task_sel: None,
                                        children: Vec::new(),
                                    });
                                    self.temp.state = UiState::Normal;
                                    // TODO: Do something more reasonable here
                                    self.topic_sel.clear();
                                }
                            }
                        }
                        UiState::AddSubtopic { name, parent_idx } => {
                            let clicked = ui.button("✔").clicked();
                            if ui.button("🗙").clicked() || ui.input().key_pressed(egui::Key::Escape)
                            {
                                self.temp.state = UiState::Normal;
                            } else {
                                ui.text_edit_singleline(name).request_focus();
                                if clicked || ui.input().key_pressed(egui::Key::Enter) {
                                    let topic = get_topic_mut(&mut self.topics, parent_idx);
                                    topic.children.push(Topic {
                                        name: name.take(),
                                        desc: String::new(),
                                        tasks: Vec::new(),
                                        task_sel: None,
                                        children: Vec::new(),
                                    });
                                    self.temp.state = UiState::Normal;
                                    // TODO: Do something more reasonable here
                                    self.topic_sel.clear();
                                }
                            }
                        }
                        _ => {
                            ui.horizontal(|ui| {
                                if ui.button("+").clicked() {
                                    self.temp.state = UiState::add_topic();
                                }
                                if ui
                                    .add_enabled(!self.topic_sel.is_empty(), egui::Button::new("-"))
                                    .clicked()
                                {
                                    if !self.topic_sel.is_empty() {
                                        remove_topic(&mut self.topics, &self.topic_sel);
                                        // TODO: Do something more reasonable
                                        self.topic_sel.clear();
                                    }
                                }
                                if let Some(topic_sel) = self.topic_sel.last_mut() {
                                    if ui.add_enabled(*topic_sel > 0, Button::new("⬆")).clicked()
                                    {
                                        self.topics.swap(*topic_sel, *topic_sel - 1);
                                        *topic_sel -= 1;
                                    }
                                    if ui
                                        .add_enabled(
                                            *topic_sel < self.topics.len() - 1,
                                            Button::new("⬇"),
                                        )
                                        .clicked()
                                    {
                                        self.topics.swap(*topic_sel, *topic_sel + 1);
                                        *topic_sel += 1;
                                    }
                                    if ui.button("Add subtopic").clicked() {
                                        self.temp.state =
                                            UiState::add_subtopic(self.topic_sel.clone());
                                    }
                                }
                            });
                        }
                    });
                    if let Some(sel) = self.topic_sel.pop() {
                        ui.heading("Topic Description");
                        ui.text_edit_multiline(&mut self.topics[sel].desc);
                        self.topic_sel.push(sel);
                    }
                });
                ui.vertical(|ui| {
                    ui.set_width(300.0);
                    ui.set_height(400.0);
                    ui.heading("Tasks");
                    if !self.topic_sel.is_empty() {
                        ScrollArea::vertical()
                            .id_source("task_scroll")
                            .show(ui, |ui| {
                                let topic = get_topic_mut(&mut self.topics, &self.topic_sel);
                                for (i, task) in topic.tasks.iter_mut().enumerate() {
                                    ui.horizontal(|ui| {
                                        ui.checkbox(&mut task.done, "");
                                        let mut text = RichText::new(&task.title);
                                        if task.done {
                                            text = text.strikethrough();
                                        }
                                        if ui
                                            .selectable_label(topic.task_sel == Some(i), text)
                                            .clicked()
                                        {
                                            topic.task_sel = Some(i);
                                        }
                                    });
                                }
                            });
                        ui.horizontal(|ui| {
                            if let UiState::AddTask(name) = &mut self.temp.state {
                                let clicked = ui.button("✔").clicked();
                                if ui.button("🗙").clicked()
                                    || ui.input().key_pressed(egui::Key::Escape)
                                {
                                    self.temp.state = UiState::Normal;
                                } else {
                                    ui.text_edit_singleline(name).request_focus();
                                    if clicked || ui.input().key_pressed(egui::Key::Enter) {
                                        get_topic_mut(&mut self.topics, &self.topic_sel)
                                            .tasks
                                            .push(Task {
                                                title: name.take(),
                                                desc: String::new(),
                                                done: false,
                                                attachments: Vec::new(),
                                            });
                                        self.temp.state = UiState::Normal;
                                        get_topic_mut(&mut self.topics, &self.topic_sel).task_sel =
                                            Some(
                                                get_topic_mut(&mut self.topics, &self.topic_sel)
                                                    .tasks
                                                    .len()
                                                    - 1,
                                            );
                                    }
                                }
                            } else {
                                if ui.button("+").clicked()
                                    || ui.input().key_pressed(egui::Key::Insert)
                                {
                                    self.temp.state = UiState::add_task();
                                }
                                if ui.button("-").clicked() {
                                    if let Some(task_sel) =
                                        get_topic_mut(&mut self.topics, &self.topic_sel).task_sel
                                    {
                                        get_topic_mut(&mut self.topics, &self.topic_sel)
                                            .tasks
                                            .remove(task_sel);
                                        if get_topic_mut(&mut self.topics, &self.topic_sel)
                                            .tasks
                                            .is_empty()
                                        {
                                            get_topic_mut(&mut self.topics, &self.topic_sel)
                                                .task_sel = None;
                                        } else {
                                            get_topic_mut(&mut self.topics, &self.topic_sel)
                                                .task_sel = Some(
                                                task_sel.clamp(
                                                    0,
                                                    get_topic_mut(
                                                        &mut self.topics,
                                                        &self.topic_sel,
                                                    )
                                                    .tasks
                                                    .len()
                                                        - 1,
                                                ),
                                            );
                                        }
                                    }
                                }
                            }
                            if let Some(task_sel) =
                                get_topic_mut(&mut self.topics, &self.topic_sel).task_sel
                            {
                                if ui.add_enabled(task_sel > 0, Button::new("⬆")).clicked() {
                                    get_topic_mut(&mut self.topics, &self.topic_sel)
                                        .tasks
                                        .swap(task_sel, task_sel - 1);
                                    get_topic_mut(&mut self.topics, &self.topic_sel).task_sel =
                                        Some(task_sel - 1);
                                }
                                if ui
                                    .add_enabled(
                                        task_sel
                                            < get_topic_mut(&mut self.topics, &self.topic_sel)
                                                .tasks
                                                .len()
                                                - 1,
                                        Button::new("⬇"),
                                    )
                                    .clicked()
                                {
                                    get_topic_mut(&mut self.topics, &self.topic_sel)
                                        .tasks
                                        .swap(task_sel, task_sel + 1);
                                    get_topic_mut(&mut self.topics, &self.topic_sel).task_sel =
                                        Some(task_sel + 1);
                                }
                            }
                        });
                        if let Some(task_sel) =
                            get_topic_mut(&mut self.topics, &self.topic_sel).task_sel
                        {
                            ui.heading("Task Description");
                            ui.text_edit_multiline(
                                &mut get_topic_mut(&mut self.topics, &self.topic_sel).tasks
                                    [task_sel]
                                    .desc,
                            );
                            for attachment in &get_topic_mut(&mut self.topics, &self.topic_sel)
                                .tasks[task_sel]
                                .attachments
                            {
                                ui.horizontal(|ui| {
                                    ui.label(attachment.filename.display().to_string());
                                    if ui.button("open").clicked() {
                                        let tmp_dir = std::env::temp_dir();
                                        let save_dir = tmp_dir.join("setodo-attachments");
                                        let path = save_dir.join(&attachment.filename);
                                        let dir_exists;
                                        if save_dir.exists() {
                                            dir_exists = true;
                                        } else {
                                            match std::fs::create_dir(save_dir) {
                                                Ok(_) => {
                                                    dir_exists = true;
                                                }
                                                Err(e) => {
                                                    error_msgbox(&format!(
                                                        "Failed to create tmp dir: {}",
                                                        e
                                                    ));
                                                    dir_exists = false;
                                                }
                                            }
                                        }
                                        if dir_exists {
                                            match std::fs::write(&path, &attachment.data) {
                                                Ok(_) => {
                                                    if let Err(e) = open::that(path) {
                                                        error_msgbox(&format!(
                                                            "Failed to open file: {}",
                                                            e
                                                        ))
                                                    }
                                                }
                                                Err(e) => error_msgbox(&format!(
                                                    "Failed to save file: {}",
                                                    e
                                                )),
                                            }
                                        }
                                    }
                                });
                            }
                            if ui.button("Attach files").clicked() {
                                if let Some(paths) = rfd::FileDialog::new().pick_files() {
                                    for path in paths {
                                        if let Some(filename) = path.file_name() {
                                            let data = std::fs::read(&path).unwrap();
                                            get_topic_mut(&mut self.topics, &self.topic_sel).tasks
                                                [task_sel]
                                                .attachments
                                                .push(Attachment {
                                                    filename: filename.into(),
                                                    data,
                                                })
                                        } else {
                                            error_msgbox(&format!(
                                                "Could not determine filename for file {:?}",
                                                path
                                            ));
                                        }
                                    }
                                }
                            }
                        }
                    }
                });
            });
        });
    }
}

fn get_topic_mut<'t>(mut topics: &'t mut [Topic], indices: &[usize]) -> &'t mut Topic {
    for i in 0..indices.len() {
        let idx = indices[i];
        if i == indices.len() - 1 {
            return &mut topics[idx];
        } else {
            topics = &mut topics[idx].children;
        }
    }
    unreachable!()
}

fn remove_topic(mut topics: &mut Vec<Topic>, indices: &[usize]) {
    for i in 0..indices.len() {
        let idx = indices[i];
        if i == indices.len() - 1 {
            topics.remove(idx);
        } else {
            topics = &mut topics[idx].children;
        }
    }
}

fn topics_ui(
    topics: &[Topic],
    cursor: &mut Vec<usize>,
    topic_sel: &mut Vec<usize>,
    ui: &mut egui::Ui,
) {
    cursor.push(0);
    for (i, topic) in topics.iter().enumerate() {
        *cursor.last_mut().unwrap() = i;
        if topic.children.is_empty() {
            if ui
                .selectable_label(*topic_sel == *cursor, &topic.name)
                .clicked()
            {
                *topic_sel = cursor.clone();
            }
        } else {
            let re = CollapsingHeader::new(&topic.name)
                .selectable(true)
                .selected(*topic_sel == *cursor)
                .show(ui, |ui| {
                    topics_ui(&topic.children, cursor, topic_sel, ui);
                });
            if re.header_response.clicked() {
                *topic_sel = cursor.clone();
            }
        }
    }
    cursor.pop();
}

fn error_msgbox(msg: &str) {
    rfd::MessageDialog::new()
        .set_level(rfd::MessageLevel::Error)
        .set_description(msg)
        .show();
}
