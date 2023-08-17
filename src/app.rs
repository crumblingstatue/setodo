use {
    crate::data::{Attachment, Task, Topic},
    eframe::{
        egui::{
            self, collapsing_header::CollapsingState, Button, Key, RichText, ScrollArea,
            TextBuffer, TextEdit,
        },
        Frame,
    },
    rmp_serde::Serializer,
    serde::{Deserialize, Serialize},
    std::{error::Error, fs::File, path::PathBuf},
};

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

enum UiState {
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
    fn move_topic_into(src_idx: Vec<usize>) -> Self {
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

    fn update(&mut self, ctx: &egui::Context, frame: &mut Frame) {
        if ctx.input(|inp| inp.key_pressed(Key::Escape)) {
            frame.close();
        }
        egui::CentralPanel::default().show(ctx, |ui| {
            let cp_avail_height = ui.available_height();
            ui.horizontal(|ui| {
                ui.set_min_height(cp_avail_height);
                ScrollArea::vertical()
                    .id_source("topics_scroll")
                    .show(ui, |ui| {
                        ui.vertical(|ui| {
                            ui.heading("Topics");
                            let any_clicked = topics_ui(
                                &mut self.topics,
                                &mut Vec::new(),
                                &mut self.topic_sel,
                                ui,
                                &mut self.temp.state,
                            );
                            ui.horizontal(|ui| match &mut self.temp.state {
                                UiState::AddTopic(name) => {
                                    let clicked = ui.button("✔").clicked();
                                    if ui.button("🗙").clicked()
                                        || ui.input(|inp| inp.key_pressed(egui::Key::Escape))
                                    {
                                        self.temp.state = UiState::Normal;
                                    } else {
                                        ui.text_edit_singleline(name).request_focus();
                                        if clicked
                                            || ui.input(|inp| inp.key_pressed(egui::Key::Enter))
                                        {
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
                                    if ui.button("🗙").clicked()
                                        || ui.input(|inp| inp.key_pressed(egui::Key::Escape))
                                    {
                                        self.temp.state = UiState::Normal;
                                    } else {
                                        ui.text_edit_singleline(name).request_focus();
                                        if clicked
                                            || ui.input(|inp| inp.key_pressed(egui::Key::Enter))
                                        {
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
                                UiState::MoveTopicInto { src_idx } => {
                                    ui.label("Click on topic to move into!");
                                    ui.label(any_clicked.to_string());
                                    if any_clicked {
                                        move_topic(&mut self.topics, src_idx, &self.topic_sel);
                                        self.temp.state = UiState::Normal;
                                    }
                                }
                                UiState::MoveTaskIntoTopic(task) => {
                                    if any_clicked {
                                        move_task_into_topic(
                                            &mut self.topics,
                                            std::mem::take(task),
                                            &self.topic_sel,
                                        );
                                        self.temp.state = UiState::Normal;
                                    }
                                }
                                _ => {
                                    ui.horizontal(|ui| {
                                        if ui.button("+").clicked() {
                                            self.temp.state = UiState::add_topic();
                                        }
                                        if ui
                                            .add_enabled(
                                                !self.topic_sel.is_empty(),
                                                egui::Button::new("-"),
                                            )
                                            .clicked()
                                            && !self.topic_sel.is_empty()
                                        {
                                            remove_topic(&mut self.topics, &self.topic_sel);
                                            // TODO: Do something more reasonable
                                            self.topic_sel.clear();
                                        }
                                        if let Some(topic_sel) = self.topic_sel.last_mut() {
                                            if ui
                                                .add_enabled(*topic_sel > 0, Button::new("⬆"))
                                                .clicked()
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
                                            if ui.button("Move topic into").clicked() {
                                                self.temp.state = UiState::move_topic_into(
                                                    self.topic_sel.clone(),
                                                );
                                            }
                                        }
                                    });
                                }
                            });
                            if !self.topic_sel.is_empty() {
                                ui.heading("Topic Description");
                                let topic = get_topic_mut(&mut self.topics, &self.topic_sel);
                                ui.text_edit_multiline(&mut topic.desc);
                            }
                        });
                    });
                let cp_avail_width = ui.available_width();
                ScrollArea::vertical()
                    .auto_shrink([false; 2])
                    .id_source("tasks_scroll")
                    .show(ui, |ui| {
                        ui.vertical(|ui| {
                            if !self.topic_sel.is_empty() {
                                let topic = get_topic_mut(&mut self.topics, &self.topic_sel);
                                ui.heading(&topic.name);
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
                                ui.horizontal(|ui| {
                                    match &mut self.temp.state {
                                        UiState::AddTask(name) => {
                                            let clicked = ui.button("✔").clicked();
                                            if ui.button("🗙").clicked()
                                                || ui
                                                    .input(|inp| inp.key_pressed(egui::Key::Escape))
                                            {
                                                self.temp.state = UiState::Normal;
                                            } else {
                                                ui.text_edit_singleline(name).request_focus();
                                                if clicked
                                                    || ui.input(|inp| {
                                                        inp.key_pressed(egui::Key::Enter)
                                                    })
                                                {
                                                    get_topic_mut(
                                                        &mut self.topics,
                                                        &self.topic_sel,
                                                    )
                                                    .tasks
                                                    .push(Task {
                                                        title: name.take(),
                                                        desc: String::new(),
                                                        done: false,
                                                        attachments: Vec::new(),
                                                    });
                                                    self.temp.state = UiState::Normal;
                                                    get_topic_mut(
                                                        &mut self.topics,
                                                        &self.topic_sel,
                                                    )
                                                    .task_sel = Some(
                                                        get_topic_mut(
                                                            &mut self.topics,
                                                            &self.topic_sel,
                                                        )
                                                        .tasks
                                                        .len()
                                                            - 1,
                                                    );
                                                }
                                            }
                                        }
                                        _ => {
                                            if ui.button("+").clicked()
                                                || ui
                                                    .input(|inp| inp.key_pressed(egui::Key::Insert))
                                            {
                                                self.temp.state = UiState::add_task();
                                            }
                                            if ui.button("-").clicked() {
                                                if let Some(task_sel) =
                                                    get_topic_mut(&mut self.topics, &self.topic_sel)
                                                        .task_sel
                                                {
                                                    get_topic_mut(
                                                        &mut self.topics,
                                                        &self.topic_sel,
                                                    )
                                                    .tasks
                                                    .remove(task_sel);
                                                    if get_topic_mut(
                                                        &mut self.topics,
                                                        &self.topic_sel,
                                                    )
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
                                        }
                                    }
                                    if let Some(task_sel) =
                                        get_topic_mut(&mut self.topics, &self.topic_sel).task_sel
                                    {
                                        if ui.add_enabled(task_sel > 0, Button::new("⬆")).clicked()
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
                                                Button::new("⬇"),
                                            )
                                            .clicked()
                                        {
                                            get_topic_mut(&mut self.topics, &self.topic_sel)
                                                .tasks
                                                .swap(task_sel, task_sel + 1);
                                            get_topic_mut(&mut self.topics, &self.topic_sel)
                                                .task_sel = Some(task_sel + 1);
                                        }
                                        if ui.button("Move task into topic").clicked() {
                                            let topic =
                                                get_topic_mut(&mut self.topics, &self.topic_sel);
                                            self.temp.state = UiState::MoveTaskIntoTopic(
                                                topic.tasks.remove(task_sel),
                                            );
                                            get_topic_mut(&mut self.topics, &self.topic_sel)
                                                .task_sel = None;
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

fn move_task_into_topic(topics: &mut [Topic], task: Task, topic_sel: &[usize]) {
    let topic = get_topic_mut(topics, topic_sel);
    topic.tasks.push(task);
}

fn move_topic(topics: &mut Vec<Topic>, src_idx: &[usize], dst_idx: &[usize]) {
    let topic = remove_topic(topics, src_idx);
    insert_topic(topics, dst_idx, topic);
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

fn remove_topic(mut topics: &mut Vec<Topic>, indices: &[usize]) -> Topic {
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

fn insert_topic(mut topics: &mut Vec<Topic>, indices: &[usize], topic: Topic) {
    for i in 0..indices.len() {
        let idx = indices[i];
        if i == indices.len() - 1 {
            topics[idx].children.push(topic);
            return;
        } else {
            topics = &mut topics[idx].children;
        }
    }
    unreachable!()
}

fn topics_ui(
    topics: &mut [Topic],
    cursor: &mut Vec<usize>,
    topic_sel: &mut Vec<usize>,
    ui: &mut egui::Ui,
    state: &mut UiState,
) -> bool {
    let mut any_clicked = false;
    cursor.push(0);
    for (i, topic) in topics.iter_mut().enumerate() {
        *cursor.last_mut().unwrap() = i;
        match state {
            UiState::RenameTopic { idx } if idx == cursor => {
                if ui.text_edit_singleline(&mut topic.name).lost_focus() {
                    *state = UiState::Normal;
                }
            }
            _ => {
                if topic.children.is_empty() {
                    let re = ui.selectable_label(*topic_sel == *cursor, &topic.name);
                    if re.clicked() {
                        any_clicked = true;
                        *topic_sel = cursor.clone();
                    }
                    if re.double_clicked() {
                        *state = UiState::RenameTopic {
                            idx: cursor.clone(),
                        }
                    }
                } else {
                    let id = ui.make_persistent_id("cheader").with(&topic.name);
                    CollapsingState::load_with_default_open(ui.ctx(), id, false)
                        .show_header(ui, |ui| {
                            let re = ui.selectable_label(*topic_sel == *cursor, &topic.name);
                            if re.clicked() {
                                *topic_sel = cursor.clone();
                                any_clicked = true;
                            }
                            if re.double_clicked() {
                                *state = UiState::RenameTopic {
                                    idx: cursor.clone(),
                                }
                            }
                        })
                        .body(|ui| {
                            any_clicked |=
                                topics_ui(&mut topic.children, cursor, topic_sel, ui, state);
                        });
                }
            }
        }
    }
    cursor.pop();
    any_clicked
}

fn error_msgbox(msg: &str) {
    rfd::MessageDialog::new()
        .set_level(rfd::MessageLevel::Error)
        .set_description(msg)
        .show();
}
