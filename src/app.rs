use crate::data::{Attachment, Task, Topic};
use eframe::{
    egui::{self, Button, Key, RichText, ScrollArea, TextBuffer},
    epi,
};
use serde::{Deserialize, Serialize};
use std::{error::Error, path::PathBuf};

#[derive(Default, Serialize, Deserialize)]
pub struct TodoApp {
    topic_sel: Option<usize>,
    adding_topic: bool,
    adding_task: bool,
    new_add_string_buf: String,
    topics: Vec<Topic>,
}

fn file_name() -> PathBuf {
    dirs::home_dir().unwrap().join(".setodo.json")
}

impl TodoApp {
    pub fn load() -> Result<Self, Box<dyn Error>> {
        let json = std::fs::read_to_string(file_name())?;
        Ok(serde_json::from_str(&json)?)
    }
    fn save(&self) -> Result<(), Box<dyn Error>> {
        let json = serde_json::to_string(self)?;
        Ok(std::fs::write(file_name(), json)?)
    }
}

impl epi::App for TodoApp {
    fn name(&self) -> &str {
        "Simple Egui Todo"
    }

    fn on_exit(&mut self) {
        TodoApp::save(self).unwrap();
    }

    fn update(&mut self, ctx: &egui::CtxRef, frame: &epi::Frame) {
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
                        for (i, topic) in self.topics.iter().enumerate() {
                            if ui
                                .selectable_label(self.topic_sel == Some(i), &topic.name)
                                .clicked()
                            {
                                self.topic_sel = Some(i);
                            }
                        }
                    });
                    ui.horizontal(|ui| {
                        if self.adding_topic {
                            let clicked = ui.button("✔").clicked();
                            if ui.button("🗙").clicked() || ui.input().key_pressed(egui::Key::Escape)
                            {
                                self.adding_topic = false;
                                self.new_add_string_buf.clear();
                            }
                            ui.text_edit_singleline(&mut self.new_add_string_buf)
                                .request_focus();
                            if clicked || ui.input().key_pressed(egui::Key::Enter) {
                                self.topics.push(Topic {
                                    name: self.new_add_string_buf.take(),
                                    desc: String::new(),
                                    tasks: Vec::new(),
                                    task_sel: None,
                                });
                                self.adding_topic = false;
                                self.topic_sel = Some(self.topics.len() - 1);
                            }
                        } else {
                            ui.horizontal(|ui| {
                                if ui.button("+").clicked() {
                                    self.adding_topic = true;
                                }
                                if ui
                                    .add_enabled(self.topic_sel.is_some(), egui::Button::new("-"))
                                    .clicked()
                                {
                                    if let Some(topic_sel) = self.topic_sel {
                                        self.topics.remove(topic_sel);
                                        if self.topics.is_empty() {
                                            self.topic_sel = None;
                                        } else {
                                            self.topic_sel =
                                                Some(topic_sel.clamp(0, self.topics.len() - 1));
                                        }
                                    }
                                }
                                if let Some(topic_sel) = self.topic_sel {
                                    if ui.add_enabled(topic_sel > 0, Button::new("⬆")).clicked() {
                                        self.topics.swap(topic_sel, topic_sel - 1);
                                        self.topic_sel = Some(topic_sel - 1);
                                    }
                                    if ui
                                        .add_enabled(
                                            topic_sel < self.topics.len() - 1,
                                            Button::new("⬇"),
                                        )
                                        .clicked()
                                    {
                                        self.topics.swap(topic_sel, topic_sel + 1);
                                        self.topic_sel = Some(topic_sel + 1);
                                    }
                                }
                            });
                        }
                    });
                    if let Some(sel) = self.topic_sel {
                        ui.heading("Topic Description");
                        ui.text_edit_multiline(&mut self.topics[sel].desc);
                    }
                });
                ui.vertical(|ui| {
                    ui.set_width(300.0);
                    ui.set_height(400.0);
                    ui.heading("Tasks");
                    if let Some(topic_sel) = self.topic_sel {
                        macro topic() {
                            self.topics[topic_sel]
                        }
                        ScrollArea::vertical()
                            .id_source("task_scroll")
                            .show(ui, |ui| {
                                let topic = &mut topic!();
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
                            if self.adding_task {
                                let clicked = ui.button("✔").clicked();
                                if ui.button("🗙").clicked()
                                    || ui.input().key_pressed(egui::Key::Escape)
                                {
                                    self.adding_task = false;
                                    self.new_add_string_buf.clear();
                                }
                                ui.text_edit_singleline(&mut self.new_add_string_buf)
                                    .request_focus();
                                if clicked || ui.input().key_pressed(egui::Key::Enter) {
                                    topic!().tasks.push(Task {
                                        title: self.new_add_string_buf.take(),
                                        desc: String::new(),
                                        done: false,
                                        attachments: Vec::new(),
                                    });
                                    self.adding_task = false;
                                    topic!().task_sel = Some(topic!().tasks.len() - 1);
                                }
                            } else {
                                if ui.button("+").clicked()
                                    || ui.input().key_pressed(egui::Key::Insert)
                                {
                                    self.adding_task = true;
                                }
                                if ui.button("-").clicked() {
                                    if let Some(task_sel) = topic!().task_sel {
                                        topic!().tasks.remove(task_sel);
                                        if topic!().tasks.is_empty() {
                                            topic!().task_sel = None;
                                        } else {
                                            topic!().task_sel =
                                                Some(task_sel.clamp(0, topic!().tasks.len() - 1));
                                        }
                                    }
                                }
                            }
                            if let Some(task_sel) = topic!().task_sel {
                                if ui.add_enabled(task_sel > 0, Button::new("⬆")).clicked() {
                                    topic!().tasks.swap(task_sel, task_sel - 1);
                                    topic!().task_sel = Some(task_sel - 1);
                                }
                                if ui
                                    .add_enabled(
                                        task_sel < topic!().tasks.len() - 1,
                                        Button::new("⬇"),
                                    )
                                    .clicked()
                                {
                                    topic!().tasks.swap(task_sel, task_sel + 1);
                                    topic!().task_sel = Some(task_sel + 1);
                                }
                            }
                        });
                        if let Some(task_sel) = topic!().task_sel {
                            ui.heading("Task Description");
                            ui.text_edit_multiline(&mut topic!().tasks[task_sel].desc);
                            for attachment in &topic!().tasks[task_sel].attachments {
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
                                            topic!().tasks[task_sel].attachments.push(Attachment {
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

fn error_msgbox(msg: &str) {
    rfd::MessageDialog::new()
        .set_level(rfd::MessageLevel::Error)
        .set_description(msg)
        .show();
}
