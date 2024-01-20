use {
    crate::{
        data::{Attachment, Task, Topic},
        ui::tree_view_ui,
    },
    eframe::{
        egui::{self, Button, Key, RichText, ScrollArea, TextBuffer, TextEdit, ViewportCommand},
        Frame,
    },
    egui_phosphor::regular as ph,
    rmp_serde::Serializer,
    serde::{Deserialize, Serialize},
    std::{error::Error, fs::File, path::PathBuf},
};

#[derive(Default, Serialize, Deserialize)]
pub struct TodoApp {
    pub topic_sel: Vec<usize>,
    pub topics: Vec<Topic>,
    #[serde(skip)]
    pub temp: TodoAppTemp,
}

/// Transient data, not saved during serialization
pub struct TodoAppTemp {
    pub state: UiState,
}

impl Default for TodoAppTemp {
    fn default() -> Self {
        Self {
            state: UiState::Normal,
        }
    }
}

pub enum UiState {
    Normal,
    AddTopic(String),
    AddSubtopic {
        name: String,
        parent_idx: Vec<usize>,
    },
    AddTask(String),
    MoveTopicInto {
        src_idx: Vec<usize>,
    },
    MoveTaskIntoTopic(Task),
    RenameTopic {
        idx: Vec<usize>,
    },
    RenameTask {
        topic_idx: Vec<usize>,
        task_idx: usize,
    },
}

impl UiState {
    pub fn add_topic() -> Self {
        Self::AddTopic(String::default())
    }
    pub fn add_subtopic(parent_idx: Vec<usize>) -> Self {
        Self::AddSubtopic {
            name: String::default(),
            parent_idx,
        }
    }
    pub fn add_task() -> Self {
        Self::AddTask(String::default())
    }
    pub fn move_topic_into(src_idx: Vec<usize>) -> Self {
        Self::MoveTopicInto { src_idx }
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

impl eframe::App for TodoApp {
    fn on_exit(&mut self, _ctx: Option<&eframe::glow::Context>) {
        TodoApp::save(self).unwrap();
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut Frame) {
        if ctx.input(|inp| inp.key_pressed(Key::Escape)) {
            ctx.send_viewport_cmd(ViewportCommand::Close);
        }
        egui::SidePanel::left("tree_view").show(ctx, |ui| tree_view_ui(ui, self));
        egui::CentralPanel::default().show(ctx, |ui| {
            let cp_avail_height = ui.available_height();
            ui.horizontal(|ui| {
                ui.set_min_height(cp_avail_height);
                let cp_avail_width = ui.available_width();
                ScrollArea::vertical()
                    .auto_shrink([false; 2])
                    .id_source("tasks_scroll")
                    .show(ui, |ui| {
                        ui.vertical(|ui| {
                            if !self.topic_sel.is_empty() {
                                let topic = get_topic_mut(&mut self.topics, &self.topic_sel);
                                ui.heading(&topic.name);
                                ui.text_edit_multiline(&mut topic.desc);
                                ui.heading("Tasks");
                                for (i, task) in topic.tasks.iter_mut().enumerate() {
                                    ui.horizontal(|ui| {
                                        ui.checkbox(&mut task.done, "");
                                        let mut text = RichText::new(&task.title);
                                        if task.done {
                                            text = text.strikethrough();
                                        }
                                        match &self.temp.state {
                                            UiState::RenameTask {
                                                task_idx,
                                                topic_idx,
                                            } if topic_idx == &self.topic_sel && i == *task_idx => {
                                                if ui
                                                    .text_edit_singleline(&mut task.title)
                                                    .lost_focus()
                                                {
                                                    self.temp.state = UiState::Normal;
                                                }
                                            }
                                            _ => {
                                                let re = ui.selectable_label(
                                                    topic.task_sel == Some(i),
                                                    text,
                                                );
                                                if re.clicked() {
                                                    topic.task_sel = Some(i);
                                                }
                                                if re.double_clicked() {
                                                    self.temp.state = UiState::RenameTask {
                                                        topic_idx: self.topic_sel.clone(),
                                                        task_idx: topic.task_sel.unwrap(),
                                                    };
                                                }
                                            }
                                        }
                                    });
                                }
                                ui.horizontal(|ui| match &mut self.temp.state {
                                    UiState::AddTask(name) => {
                                        let clicked = ui.button(ph::CHECK_FAT).clicked();
                                        if ui.button(ph::X_CIRCLE).clicked()
                                            || ui.input(|inp| inp.key_pressed(egui::Key::Escape))
                                        {
                                            self.temp.state = UiState::Normal;
                                        } else {
                                            ui.text_edit_singleline(name).request_focus();
                                            if clicked
                                                || ui.input(|inp| inp.key_pressed(egui::Key::Enter))
                                            {
                                                let topic = get_topic_mut(
                                                    &mut self.topics,
                                                    &self.topic_sel,
                                                );
                                                topic.tasks.insert(
                                                    topic.task_sel.map(|idx| idx + 1).unwrap_or(0),
                                                    Task {
                                                        title: name.take(),
                                                        desc: String::new(),
                                                        done: false,
                                                        attachments: Vec::new(),
                                                    },
                                                );
                                                self.temp.state = UiState::Normal;
                                                match &mut topic.task_sel {
                                                    Some(sel) => {
                                                        if *sel + 1 < topic.tasks.len() {
                                                            *sel += 1;
                                                        }
                                                    }
                                                    None => {
                                                        topic.task_sel = Some(0);
                                                    }
                                                }
                                            }
                                        }
                                    }
                                    _ => {
                                        if ui.button(ph::FILE_PLUS).clicked()
                                            || ui.input(|inp| inp.key_pressed(egui::Key::Insert))
                                        {
                                            self.temp.state = UiState::add_task();
                                        }
                                        if ui.button(ph::TRASH).clicked() {
                                            if let Some(task_sel) =
                                                get_topic_mut(&mut self.topics, &self.topic_sel)
                                                    .task_sel
                                            {
                                                get_topic_mut(&mut self.topics, &self.topic_sel)
                                                    .tasks
                                                    .remove(task_sel);
                                                if get_topic_mut(&mut self.topics, &self.topic_sel)
                                                    .tasks
                                                    .is_empty()
                                                {
                                                    get_topic_mut(
                                                        &mut self.topics,
                                                        &self.topic_sel,
                                                    )
                                                    .task_sel = None;
                                                } else {
                                                    get_topic_mut(
                                                        &mut self.topics,
                                                        &self.topic_sel,
                                                    )
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
                                        if let Some(task_sel) =
                                            get_topic_mut(&mut self.topics, &self.topic_sel)
                                                .task_sel
                                        {
                                            if ui
                                                .add_enabled(
                                                    task_sel > 0,
                                                    Button::new(ph::ARROW_FAT_UP),
                                                )
                                                .clicked()
                                            {
                                                get_topic_mut(&mut self.topics, &self.topic_sel)
                                                    .tasks
                                                    .swap(task_sel, task_sel - 1);
                                                get_topic_mut(&mut self.topics, &self.topic_sel)
                                                    .task_sel = Some(task_sel - 1);
                                            }
                                            if ui
                                                .add_enabled(
                                                    task_sel
                                                        < get_topic_mut(
                                                            &mut self.topics,
                                                            &self.topic_sel,
                                                        )
                                                        .tasks
                                                        .len()
                                                            - 1,
                                                    Button::new(ph::ARROW_FAT_DOWN),
                                                )
                                                .clicked()
                                            {
                                                get_topic_mut(&mut self.topics, &self.topic_sel)
                                                    .tasks
                                                    .swap(task_sel, task_sel + 1);
                                                get_topic_mut(&mut self.topics, &self.topic_sel)
                                                    .task_sel = Some(task_sel + 1);
                                            }
                                            if ui
                                                .button(ph::SORT_DESCENDING)
                                                .on_hover_text("Auto sort")
                                                .clicked()
                                            {
                                                get_topic_mut(&mut self.topics, &self.topic_sel)
                                                    .tasks
                                                    .sort_by(|a, b| {
                                                        a.done
                                                            .cmp(&b.done)
                                                            .then_with(|| a.title.cmp(&b.title))
                                                    });
                                            }
                                            if ui.button("Move task into topic").clicked() {
                                                let topic = get_topic_mut(
                                                    &mut self.topics,
                                                    &self.topic_sel,
                                                );
                                                self.temp.state = UiState::MoveTaskIntoTopic(
                                                    topic.tasks.remove(task_sel),
                                                );
                                                get_topic_mut(&mut self.topics, &self.topic_sel)
                                                    .task_sel = None;
                                            }
                                        }
                                    }
                                });
                                ui.separator();
                                if let Some(task_sel) =
                                    get_topic_mut(&mut self.topics, &self.topic_sel).task_sel
                                {
                                    let task =
                                        &mut get_topic_mut(&mut self.topics, &self.topic_sel).tasks
                                            [task_sel];
                                    ui.heading(&task.title);
                                    let te = TextEdit::multiline(&mut task.desc)
                                        .desired_width(cp_avail_width);
                                    ui.add(te);
                                    for attachment in
                                        &get_topic_mut(&mut self.topics, &self.topic_sel).tasks
                                            [task_sel]
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
                                                    get_topic_mut(&mut self.topics, &self.topic_sel)
                                                        .tasks[task_sel]
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
        });
    }
}

pub fn move_task_into_topic(topics: &mut [Topic], task: Task, topic_sel: &[usize]) {
    let topic = get_topic_mut(topics, topic_sel);
    topic.tasks.push(task);
}

pub fn move_topic(topics: &mut Vec<Topic>, src_idx: &[usize], dst_idx: &[usize]) {
    let topic = remove_topic(topics, src_idx);
    insert_topic(topics, dst_idx, topic);
}

pub fn get_topic_mut<'t>(mut topics: &'t mut [Topic], indices: &[usize]) -> &'t mut Topic {
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

pub fn remove_topic(mut topics: &mut Vec<Topic>, indices: &[usize]) -> Topic {
    for i in 0..indices.len() {
        let idx = indices[i];
        if i == indices.len() - 1 {
            return topics.remove(idx);
        } else {
            topics = &mut topics[idx].children;
        }
    }
    unreachable!()
}

pub fn insert_topic(mut topics: &mut Vec<Topic>, indices: &[usize], topic: Topic) {
    for i in 0..indices.len() {
        let idx = indices[i];
        if i == indices.len() - 1 {
            topics[idx].children.push(topic);
            return;
        } else {
            topics = &mut topics[idx].children;
        }
    }
}

pub fn error_msgbox(msg: &str) {
    rfd::MessageDialog::new()
        .set_level(rfd::MessageLevel::Error)
        .set_description(msg)
        .show();
}
