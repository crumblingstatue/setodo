use {
    crate::{
        app::{get_topic_mut, move_task_into_topic, move_topic, remove_topic, TodoApp, UiState},
        data::Topic,
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
