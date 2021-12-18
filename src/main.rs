#![feature(decl_macro)]

use std::{error::Error, path::PathBuf};

use eframe::{
    egui::{self, ScrollArea, TextBuffer},
    epi,
};
use serde::{Deserialize, Serialize};

#[derive(Default, Serialize, Deserialize)]
struct TodoApp {
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
    fn load() -> Result<Self, Box<dyn Error>> {
        let json = std::fs::read_to_string(file_name())?;
        Ok(serde_json::from_str(&json)?)
    }
    fn save(&self) -> Result<(), Box<dyn Error>> {
        let json = serde_json::to_string(self)?;
        Ok(std::fs::write(file_name(), json)?)
    }
}

#[derive(Serialize, Deserialize)]
struct Topic {
    name: String,
    desc: String,
    tasks: Vec<Task>,
    /// Each topic remembers what task was last selected
    task_sel: Option<usize>,
}

#[derive(Serialize, Deserialize)]
struct Task {
    title: String,
    desc: String,
}

impl epi::App for TodoApp {
    fn name(&self) -> &str {
        "Simple Egui Todo"
    }

    fn on_exit(&mut self) {
        TodoApp::save(self).unwrap();
    }

    fn update(&mut self, ctx: &egui::CtxRef, _frame: &mut epi::Frame<'_>) {
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
                                for (i, task) in topic.tasks.iter().enumerate() {
                                    if ui
                                        .selectable_label(topic.task_sel == Some(i), &task.title)
                                        .clicked()
                                    {
                                        topic.task_sel = Some(i);
                                    }
                                }
                            });
                        ui.horizontal(|ui| {
                            if self.adding_task {
                                let clicked = ui.button("✔").clicked();
                                ui.text_edit_singleline(&mut self.new_add_string_buf)
                                    .request_focus();
                                if clicked || ui.input().key_pressed(egui::Key::Enter) {
                                    topic!().tasks.push(Task {
                                        title: self.new_add_string_buf.take(),
                                        desc: String::new(),
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
                        });
                        if let Some(task_sel) = topic!().task_sel {
                            ui.heading("Task Description");
                            ui.text_edit_multiline(&mut topic!().tasks[task_sel].desc);
                        }
                    }
                });
            });
        });
    }
}

fn main() {
    let app = match TodoApp::load() {
        Ok(app) => app,
        Err(e) => {
            eprintln!("{}", e);
            TodoApp::default()
        }
    };
    let native_options = eframe::NativeOptions::default();
    eframe::run_native(Box::new(app), native_options);
}
