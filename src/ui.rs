use {
    crate::{
        app::{get_topic_mut, move_task_into_topic, move_topic, remove_topic, TodoApp, UiState},
        data::{Attachment, Task, Topic},
    },
    eframe::egui::{self, collapsing_header::CollapsingState, ScrollArea, TextBuffer},
    egui_phosphor::regular as ph,
};

pub fn tree_view_ui(ui: &mut egui::Ui, app: &mut TodoApp) {
    ScrollArea::vertical()
        .id_source("topics_scroll")
        .show(ui, |ui| {
            ui.vertical(|ui| {
                ui.heading("Topics");
                let any_clicked = topics_ui(
                    &mut app.topics,
                    &mut Vec::new(),
                    &mut app.topic_sel,
                    ui,
                    &mut app.temp.state,
                );
                ui.horizontal(|ui| match &mut app.temp.state {
                    UiState::AddTopic(name) => {
                        let clicked = ui.button(ph::CHECK_FAT).clicked();
                        if ui.button(ph::X_CIRCLE).clicked()
                            || ui.input(|inp| inp.key_pressed(egui::Key::Escape))
                        {
                            app.temp.state = UiState::Normal;
                        } else {
                            ui.text_edit_singleline(name).request_focus();
                            if clicked || ui.input(|inp| inp.key_pressed(egui::Key::Enter)) {
                                app.topics.push(Topic {
                                    name: name.take(),
                                    desc: String::new(),
                                    tasks: Vec::new(),
                                    task_sel: None,
                                    children: Vec::new(),
                                });
                                app.temp.state = UiState::Normal;
                                // TODO: Do something more reasonable here
                                app.topic_sel.clear();
                            }
                        }
                    }
                    UiState::AddSubtopic { name, parent_idx } => {
                        let clicked = ui.button(ph::CHECK_FAT).clicked();
                        if ui.button(ph::X_CIRCLE).clicked()
                            || ui.input(|inp| inp.key_pressed(egui::Key::Escape))
                        {
                            app.temp.state = UiState::Normal;
                        } else {
                            ui.text_edit_singleline(name).request_focus();
                            if clicked || ui.input(|inp| inp.key_pressed(egui::Key::Enter)) {
                                let topic = get_topic_mut(&mut app.topics, parent_idx);
                                topic.children.push(Topic {
                                    name: name.take(),
                                    desc: String::new(),
                                    tasks: Vec::new(),
                                    task_sel: None,
                                    children: Vec::new(),
                                });
                                app.temp.state = UiState::Normal;
                                // TODO: Do something more reasonable here
                                app.topic_sel.clear();
                            }
                        }
                    }
                    UiState::MoveTopicInto { src_idx } => {
                        ui.label("Click on topic to move into!");
                        ui.label(any_clicked.to_string());
                        if any_clicked {
                            move_topic(&mut app.topics, src_idx, &app.topic_sel);
                            app.temp.state = UiState::Normal;
                        }
                    }
                    UiState::MoveTaskIntoTopic(task) => {
                        if any_clicked {
                            move_task_into_topic(
                                &mut app.topics,
                                std::mem::take(task),
                                &app.topic_sel,
                            );
                            app.temp.state = UiState::Normal;
                        }
                    }
                    _ => {
                        ui.horizontal(|ui| {
                            if ui.button(ph::FILE_PLUS).clicked() {
                                app.temp.state = UiState::add_topic();
                            }
                            if ui
                                .add_enabled(
                                    !app.topic_sel.is_empty(),
                                    egui::Button::new(ph::TRASH),
                                )
                                .clicked()
                                && !app.topic_sel.is_empty()
                            {
                                remove_topic(&mut app.topics, &app.topic_sel);
                                // TODO: Do something more reasonable
                                app.topic_sel.clear();
                            }
                            if let Some(topic_sel) = app.topic_sel.last_mut() {
                                if ui
                                    .add_enabled(
                                        *topic_sel > 0,
                                        egui::Button::new(ph::ARROW_FAT_UP),
                                    )
                                    .clicked()
                                {
                                    app.topics.swap(*topic_sel, *topic_sel - 1);
                                    *topic_sel -= 1;
                                }
                                if ui
                                    .add_enabled(
                                        *topic_sel < app.topics.len() - 1,
                                        egui::Button::new(ph::ARROW_FAT_DOWN),
                                    )
                                    .clicked()
                                {
                                    app.topics.swap(*topic_sel, *topic_sel + 1);
                                    *topic_sel += 1;
                                }
                                if ui.button("Add subtopic").clicked() {
                                    app.temp.state = UiState::add_subtopic(app.topic_sel.clone());
                                }
                                if ui.button("Move topic into").clicked() {
                                    app.temp.state =
                                        UiState::move_topic_into(app.topic_sel.clone());
                                }
                            }
                        });
                    }
                });
            });
        });
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

pub fn central_panel_ui(ui: &mut egui::Ui, app: &mut TodoApp) {
    let cp_avail_height = ui.available_height();
    ui.horizontal(|ui| {
        ui.set_min_height(cp_avail_height);
        let cp_avail_width = ui.available_width();
        ScrollArea::vertical()
            .auto_shrink([false; 2])
            .id_source("tasks_scroll")
            .show(ui, |ui| {
                ui.vertical(|ui| {
                    if !app.topic_sel.is_empty() {
                        let topic = get_topic_mut(&mut app.topics, &app.topic_sel);
                        ui.heading(&topic.name);
                        ui.text_edit_multiline(&mut topic.desc);
                        tasks_list_ui(ui, app);
                        if let Some(task_sel) =
                            get_topic_mut(&mut app.topics, &app.topic_sel).task_sel
                        {
                            ui.separator();
                            task_ui(app, task_sel, ui, cp_avail_width);
                        }
                    }
                });
            });
    });
}

fn tasks_list_ui(ui: &mut egui::Ui, app: &mut TodoApp) {
    let topic = get_topic_mut(&mut app.topics, &app.topic_sel);
    ui.heading("Tasks");
    for (i, task) in topic.tasks.iter_mut().enumerate() {
        ui.horizontal(|ui| {
            ui.checkbox(&mut task.done, "");
            let mut text = egui::RichText::new(&task.title);
            if task.done {
                text = text.strikethrough();
            }
            match &app.temp.state {
                UiState::RenameTask {
                    task_idx,
                    topic_idx,
                } if topic_idx == &app.topic_sel && i == *task_idx => {
                    if ui.text_edit_singleline(&mut task.title).lost_focus() {
                        app.temp.state = UiState::Normal;
                    }
                }
                _ => {
                    let re = ui.selectable_label(topic.task_sel == Some(i), text);
                    if re.clicked() {
                        topic.task_sel = Some(i);
                    }
                    if re.double_clicked() {
                        app.temp.state = UiState::RenameTask {
                            topic_idx: app.topic_sel.clone(),
                            task_idx: topic.task_sel.unwrap(),
                        };
                    }
                }
            }
        });
    }
    ui.horizontal(|ui| match &mut app.temp.state {
        UiState::AddTask(name) => {
            let clicked = ui.button(ph::CHECK_FAT).clicked();
            if ui.button(ph::X_CIRCLE).clicked()
                || ui.input(|inp| inp.key_pressed(egui::Key::Escape))
            {
                app.temp.state = UiState::Normal;
            } else {
                ui.text_edit_singleline(name).request_focus();
                if clicked || ui.input(|inp| inp.key_pressed(egui::Key::Enter)) {
                    let topic = get_topic_mut(&mut app.topics, &app.topic_sel);
                    topic.tasks.insert(
                        topic.task_sel.map(|idx| idx + 1).unwrap_or(0),
                        Task {
                            title: name.take(),
                            desc: String::new(),
                            done: false,
                            attachments: Vec::new(),
                        },
                    );
                    app.temp.state = UiState::Normal;
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
                app.temp.state = UiState::add_task();
            }
            if ui.button(ph::TRASH).clicked() {
                if let Some(task_sel) = get_topic_mut(&mut app.topics, &app.topic_sel).task_sel {
                    get_topic_mut(&mut app.topics, &app.topic_sel)
                        .tasks
                        .remove(task_sel);
                    if get_topic_mut(&mut app.topics, &app.topic_sel)
                        .tasks
                        .is_empty()
                    {
                        get_topic_mut(&mut app.topics, &app.topic_sel).task_sel = None;
                    } else {
                        get_topic_mut(&mut app.topics, &app.topic_sel).task_sel =
                            Some(task_sel.clamp(
                                0,
                                get_topic_mut(&mut app.topics, &app.topic_sel).tasks.len() - 1,
                            ));
                    }
                }
            }
            if let Some(task_sel) = get_topic_mut(&mut app.topics, &app.topic_sel).task_sel {
                if ui
                    .add_enabled(task_sel > 0, egui::Button::new(ph::ARROW_FAT_UP))
                    .clicked()
                {
                    get_topic_mut(&mut app.topics, &app.topic_sel)
                        .tasks
                        .swap(task_sel, task_sel - 1);
                    get_topic_mut(&mut app.topics, &app.topic_sel).task_sel = Some(task_sel - 1);
                }
                if ui
                    .add_enabled(
                        task_sel < get_topic_mut(&mut app.topics, &app.topic_sel).tasks.len() - 1,
                        egui::Button::new(ph::ARROW_FAT_DOWN),
                    )
                    .clicked()
                {
                    get_topic_mut(&mut app.topics, &app.topic_sel)
                        .tasks
                        .swap(task_sel, task_sel + 1);
                    get_topic_mut(&mut app.topics, &app.topic_sel).task_sel = Some(task_sel + 1);
                }
                if ui
                    .button(ph::SORT_DESCENDING)
                    .on_hover_text("Auto sort")
                    .clicked()
                {
                    get_topic_mut(&mut app.topics, &app.topic_sel)
                        .tasks
                        .sort_by(|a, b| a.done.cmp(&b.done).then_with(|| a.title.cmp(&b.title)));
                }
                if ui.button("Move task into topic").clicked() {
                    let topic = get_topic_mut(&mut app.topics, &app.topic_sel);
                    app.temp.state = UiState::MoveTaskIntoTopic(topic.tasks.remove(task_sel));
                    get_topic_mut(&mut app.topics, &app.topic_sel).task_sel = None;
                }
            }
        }
    });
}

/// UI for details about an individual task
fn task_ui(app: &mut TodoApp, task_sel: usize, ui: &mut egui::Ui, cp_avail_width: f32) {
    let task = &mut get_topic_mut(&mut app.topics, &app.topic_sel).tasks[task_sel];
    ui.heading(&task.title);
    let te = egui::TextEdit::multiline(&mut task.desc).desired_width(cp_avail_width);
    ui.add(te);
    for attachment in &get_topic_mut(&mut app.topics, &app.topic_sel).tasks[task_sel].attachments {
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
                            error_msgbox(&format!("Failed to create tmp dir: {}", e));
                            dir_exists = false;
                        }
                    }
                }
                if dir_exists {
                    match std::fs::write(&path, &attachment.data) {
                        Ok(_) => {
                            if let Err(e) = open::that(path) {
                                error_msgbox(&format!("Failed to open file: {}", e))
                            }
                        }
                        Err(e) => error_msgbox(&format!("Failed to save file: {}", e)),
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
                    get_topic_mut(&mut app.topics, &app.topic_sel).tasks[task_sel]
                        .attachments
                        .push(Attachment {
                            filename: filename.into(),
                            data,
                        })
                } else {
                    error_msgbox(&format!("Could not determine filename for file {:?}", path));
                }
            }
        }
    }
}

pub fn error_msgbox(msg: &str) {
    rfd::MessageDialog::new()
        .set_level(rfd::MessageLevel::Error)
        .set_description(msg)
        .show();
}
