use {
    crate::{
        app::{
            ActionFlags, ModalPayload, StoredFontData, TodoApp, TodoAppTemp, UiState,
            move_task_into_topic,
        },
        cmd::Cmd,
        data::{Attachment, Entry, EntryKind, Topic},
        tree,
    },
    constcat::concat as cc,
    eframe::egui::{
        self, ScrollArea, TextBuffer, ViewportCommand, collapsing_header::CollapsingState,
    },
    egui_commonmark::CommonMarkViewer,
    egui_fontcfg::FontDefsUiMsg,
    egui_phosphor::regular as ph,
};

pub fn tree_view_ui(ui: &mut egui::Ui, app: &mut TodoApp) {
    tree_view_top_bar(ui, app);
    ui.separator();
    let mut any_clicked = false;
    ScrollArea::vertical()
        .max_height(ui.available_height() - 36.0)
        .auto_shrink(false)
        .id_salt("topics_scroll")
        .show(ui, |ui| {
            ui.vertical(|ui| {
                if !app.temp.find_string.is_empty() {
                    find_ui(ui, app);
                    return;
                }
                let root_label_text = if app.temp.per_dirty {
                    egui::RichText::new("Topics*").color(egui::Color32::YELLOW)
                } else {
                    egui::RichText::new("Topics")
                };
                let mut re = ui.selectable_label(app.per.topic_sel.is_empty(), root_label_text);
                if app.temp.per_dirty {
                    re = re.on_hover_ui(|ui| {
                        ui.label(
                            egui::RichText::new("There are unsaved changes")
                                .color(egui::Color32::YELLOW),
                        );
                    });
                }
                if re.clicked() {
                    any_clicked = true;
                    app.per.topic_sel.clear();
                }
                ui.indent("root_indent", |ui| {
                    any_clicked |= topics_ui(
                        &mut app.per.topics,
                        &mut Vec::new(),
                        &mut app.per.topic_sel,
                        ui,
                        &mut app.temp.state,
                        &mut app.temp.per_dirty,
                        &mut app.temp.action_flags,
                        &mut app.temp.cmd,
                    );
                });
            });
        });
    ui.separator();
    tree_view_bottom_bar(ui, app, any_clicked);
}

fn tree_view_bottom_bar(ui: &mut egui::Ui, app: &mut TodoApp, any_clicked: bool) {
    ui.horizontal(|ui| match &mut app.temp.state {
        UiState::AddSubtopic { name, parent_idx } => {
            let clicked = ui.button(ph::CHECK_FAT).clicked();
            if ui.button(ph::X_CIRCLE).clicked()
                || ui.input(|inp| inp.key_pressed(egui::Key::Escape))
            {
                app.temp.state = UiState::Normal;
            } else {
                ui.text_edit_singleline(name).request_focus();
                if clicked || ui.input(|inp| inp.key_pressed(egui::Key::Enter)) {
                    let topic_list = match tree::get_mut(&mut app.per.topics, parent_idx) {
                        Some(topic) => &mut topic.children,
                        None => &mut app.per.topics,
                    };
                    topic_list.push(Topic {
                        name: name.take(),
                        desc: String::new(),
                        entries: Vec::new(),
                        task_sel: None,
                        children: Vec::new(),
                    });
                    let mut new_sel = parent_idx.to_vec();
                    new_sel.push(topic_list.len() - 1);
                    app.temp.state = UiState::Normal;
                    // TODO: Do something more reasonable here
                    app.per.topic_sel = new_sel;
                    app.temp.per_dirty = true;
                }
            }
        }
        UiState::MoveTopicInto { src_idx } => {
            ui.label("Click on topic to move into!");
            if any_clicked {
                tree::move_(&mut app.per.topics, src_idx, &app.per.topic_sel);
                app.temp.state = UiState::Normal;
            }
            if ui.button("Cancel").clicked() {
                app.temp.state = UiState::Normal;
            }
        }
        UiState::MoveTaskIntoTopic(task) => {
            if any_clicked {
                let result = move_task_into_topic(
                    &mut app.per.topics,
                    std::mem::take(task),
                    &app.per.topic_sel,
                );
                match result {
                    Ok(()) => app.temp.state = UiState::Normal,
                    Err(()) => eprintln!("Failed to move task into topic"),
                }
            }
        }
        _ => {
            ui.horizontal(|ui| {
                if ui
                    .button(ph::FILE_PLUS)
                    .on_hover_text("New topic")
                    .clicked()
                {
                    app.temp.state = UiState::add_subtopic(app.per.topic_sel.clone());
                }
                if ui
                    .add_enabled(!app.per.topic_sel.is_empty(), egui::Button::new(ph::TRASH))
                    .on_hover_text("Delete topic")
                    .clicked()
                    && !app.per.topic_sel.is_empty()
                {
                    tree::remove(&mut app.per.topics, &app.per.topic_sel);
                    // TODO: Do something more reasonable
                    app.per.topic_sel.clear();
                }
                if let Some((last, first_chunk)) = app.per.topic_sel.split_last_mut() {
                    let topics = if first_chunk.is_empty() {
                        &mut app.per.topics
                    } else {
                        match tree::get_mut(&mut app.per.topics, first_chunk) {
                            Some(topic) => &mut topic.children,
                            None => {
                                ui.label("TODO: Bug (probably)");
                                return;
                            }
                        }
                    };
                    if ui
                        .add_enabled(*last > 0, egui::Button::new(ph::ARROW_FAT_UP))
                        .on_hover_text("Move topic up")
                        .clicked()
                    {
                        topics.swap(*last, *last - 1);
                        *last -= 1;
                    }
                    if ui
                        .add_enabled(
                            *last < topics.len().saturating_sub(1),
                            egui::Button::new(ph::ARROW_FAT_DOWN),
                        )
                        .on_hover_text("Move topic down")
                        .clicked()
                    {
                        topics.swap(*last, *last + 1);
                        *last += 1;
                    }
                    if ui
                        .button("⬈ Move")
                        .on_hover_text("Move topic inside another topic")
                        .clicked()
                    {
                        app.temp.state = UiState::move_topic_into(app.per.topic_sel.clone());
                    }
                }
            });
        }
    });
}

fn tree_view_top_bar(ui: &mut egui::Ui, app: &mut TodoApp) {
    let esc_pressed = ui.input(|inp| inp.key_pressed(egui::Key::Escape));
    ui.horizontal(|ui| {
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.menu_button("☰ Menu", |ui| {
                if ui
                    .add_enabled(
                        app.temp.per_dirty,
                        egui::Button::new("💾 Save").shortcut_text("Ctrl+S"),
                    )
                    .clicked()
                {
                    if let Err(e) = app.save_persistent() {
                        eprintln!("Error when saving: {e}");
                    }
                    ui.close_menu();
                }
                if ui
                    .add_enabled(
                        app.temp.per_dirty,
                        egui::Button::new("⟲ Reload").shortcut_text("Ctrl+R"),
                    )
                    .clicked()
                {
                    if let Err(e) = app.reload_persistent() {
                        eprintln!("Reload error: {e}");
                    }
                    ui.close_menu();
                }
                ui.separator();
                if ui
                    .button(cc!(ph::ARROW_BEND_LEFT_UP, " Collapse all"))
                    .clicked()
                {
                    app.temp.action_flags.collapse_all = true;
                    ui.close_menu();
                }
                if ui
                    .button(cc!(ph::ARROW_BEND_RIGHT_DOWN, " Expand all"))
                    .clicked()
                {
                    app.temp.action_flags.expand_all = true;
                    ui.close_menu();
                }
                ui.separator();
                if ui.button("🗛 Font config").clicked() {
                    app.temp.state = UiState::FontCfg;
                    ui.close_menu();
                }
                ui.separator();
                if ui.button(cc!(ph::DOOR_OPEN, " Save & Quit")).clicked() {
                    ui.ctx().send_viewport_cmd(ViewportCommand::Close);
                    ui.close_menu();
                }
            });
            if ui
                .button("👁 Hide")
                .on_hover_text("Hotkey: Esc\nAlso autosaves.")
                .clicked()
            {
                ui.ctx()
                    .send_viewport_cmd(egui::ViewportCommand::Visible(false));
                if let Err(e) = app.save_persistent() {
                    eprintln!("Autosave error: {e}");
                }
            };
            let re = ui.add(
                egui::TextEdit::singleline(&mut app.temp.find_string).hint_text("🔍 Find (ctrl+F)"),
            );
            if ui.input(|inp| inp.modifiers.ctrl && inp.key_pressed(egui::Key::F)) {
                re.request_focus();
            }
            if !app.temp.find_string.is_empty() && esc_pressed {
                app.temp.esc_was_used = true;
                app.temp.find_string.clear();
            }
        });
    });
}

fn find_ui(ui: &mut egui::Ui, app: &mut TodoApp) {
    let find_str = &app.temp.find_string;
    for matched_topic in collect_matches(&app.per.topics, find_str) {
        let Some(topic) = tree::get_mut(&mut app.per.topics, &matched_topic.cursor) else {
            ui.label(format!("<error indexing: ({:?}()>", matched_topic.cursor));
            continue;
        };
        if ui.link(&topic.name).clicked() {
            app.per.topic_sel = matched_topic.cursor;
            return;
        }
        ui.indent("find_ui_indent", |ui| {
            for en_idx in matched_topic.matching_entries {
                if ui.link(&topic.entries[en_idx].title).clicked() {
                    app.per.topic_sel = matched_topic.cursor;
                    topic.task_sel = Some(en_idx);
                    return;
                }
            }
        });
    }
}

struct MatchingTopic {
    cursor: Vec<usize>,
    matching_entries: Vec<usize>,
}

fn collect_matches(topics: &[Topic], find_str: &str) -> Vec<MatchingTopic> {
    let mut matches = Vec::new();
    let cursor = vec![];
    collect_matches_inner(topics, find_str, cursor, &mut matches);
    matches
}

fn collect_matches_inner(
    topics: &[Topic],
    find_str: &str,
    cursor: Vec<usize>,
    matches: &mut Vec<MatchingTopic>,
) {
    for (i, topic) in topics.iter().enumerate() {
        let mut new_cursor = cursor.clone();
        new_cursor.push(i);
        if let Some(topic) = matching_topic(topic, find_str, new_cursor.clone()) {
            matches.push(topic);
        }
        collect_matches_inner(&topic.children, find_str, new_cursor, matches);
    }
}

fn matching_topic(topic: &Topic, find_str: &str, cursor: Vec<usize>) -> Option<MatchingTopic> {
    let find_lower = &find_str.to_ascii_lowercase();
    let mut is_match = false;
    is_match |= topic.name.to_ascii_lowercase().contains(find_lower)
        || topic.desc.to_ascii_lowercase().contains(find_lower);
    let mut matching_entries = Vec::new();
    for (i, en) in topic.entries.iter().enumerate() {
        if en.title.to_ascii_lowercase().contains(find_lower)
            || en.desc.to_ascii_lowercase().contains(find_lower)
        {
            matching_entries.push(i);
        }
    }
    is_match |= !matching_entries.is_empty();
    is_match.then_some(MatchingTopic {
        cursor,
        matching_entries,
    })
}

#[expect(clippy::too_many_arguments)]
fn topics_ui(
    topics: &mut [Topic],
    cursor: &mut Vec<usize>,
    topic_sel: &mut Vec<usize>,
    ui: &mut egui::Ui,
    state: &mut UiState,
    per_dirty: &mut bool,
    action_flags: &mut ActionFlags,
    cmd: &mut Vec<Cmd>,
) -> bool {
    let mut any_clicked = false;
    cursor.push(0);
    for (i, topic) in topics.iter_mut().enumerate() {
        *cursor.last_mut().unwrap() = i;
        match state {
            UiState::RenameTopic { idx } if idx == cursor => {
                let re = ui.text_edit_singleline(&mut topic.name);
                cmd.retain(|cmd| {
                    if let Cmd::FocusTextEdit = cmd {
                        re.request_focus();
                        false
                    } else {
                        true
                    }
                });
                if re.lost_focus() {
                    *state = UiState::Normal;
                }
                if re.changed() {
                    *per_dirty = true;
                }
            }
            _ => {
                macro_rules! ctx_menu {
                    () => {
                        |ui: &mut egui::Ui| {
                            if ui.button(cc!(ph::NOTE_PENCIL, " Rename topic")).clicked() {
                                *state = UiState::RenameTopic {
                                    idx: cursor.clone(),
                                };
                                cmd.push(Cmd::FocusTextEdit);
                                ui.close_menu();
                            }
                            if ui.button(cc!(ph::FILE_PLUS, " Create subtopic")).clicked() {
                                topic.children.push(Topic::new_unnamed());
                                ui.close_menu();
                            }
                            if ui.button(cc!(ph::TRASH, " Delete topic")).clicked() {
                                cmd.push(Cmd::RemoveTopic {
                                    idx: cursor.clone(),
                                });
                                ui.close_menu();
                            }
                        }
                    };
                }
                if topic.children.is_empty() {
                    let re = ui.selectable_label(*topic_sel == *cursor, &topic.name);
                    if re.clicked() {
                        any_clicked = true;
                        topic_sel.clone_from(cursor);
                    }
                    if re.double_clicked() {
                        *state = UiState::RenameTopic {
                            idx: cursor.clone(),
                        }
                    }
                    re.context_menu(ctx_menu!());
                } else {
                    let id = ui.make_persistent_id("cheader").with(&topic.name);
                    let mut cs = CollapsingState::load_with_default_open(ui.ctx(), id, false);
                    if action_flags.collapse_all {
                        cs.set_open(false);
                    }
                    if action_flags.expand_all {
                        cs.set_open(true);
                    }
                    cs.show_header(ui, |ui| {
                        let re = ui.selectable_label(*topic_sel == *cursor, &topic.name);
                        if re.clicked() {
                            topic_sel.clone_from(cursor);
                            any_clicked = true;
                        }
                        if re.double_clicked() {
                            *state = UiState::RenameTopic {
                                idx: cursor.clone(),
                            }
                        }
                        re.context_menu(ctx_menu!());
                    })
                    .body(|ui| {
                        any_clicked |= topics_ui(
                            &mut topic.children,
                            cursor,
                            topic_sel,
                            ui,
                            state,
                            per_dirty,
                            action_flags,
                            cmd,
                        );
                    });
                }
            }
        }
    }
    cursor.pop();
    any_clicked
}

pub fn central_panel_ui(ui: &mut egui::Ui, app: &mut TodoApp) {
    if matches!(app.temp.state, UiState::FontCfg) {
        if ui.link("Back").clicked() {
            app.temp.state = UiState::Normal;
        }
        ui.separator();
        if let FontDefsUiMsg::SaveRequest = app.temp.font_defs_ui.show(
            ui,
            &mut app.temp.font_defs_edit_copy,
            Some(&mut app.temp.custom_edit_copy),
        ) {
            app.per.stored_font_data = Some(StoredFontData {
                families: app.temp.font_defs_edit_copy.families.clone(),
                custom: app.temp.custom_edit_copy.clone(),
            });
        }
        return;
    }
    let cp_avail_height = ui.available_height();
    ui.horizontal(|ui| {
        ui.set_min_height(cp_avail_height);
        let cp_avail_width = ui.available_width();
        ui.vertical(|ui| {
            if !app.per.topic_sel.is_empty() {
                let Some(topic) = tree::get_mut(&mut app.per.topics, &app.per.topic_sel) else {
                    ui.label(format!(
                        "<error getting topic. index: {:?}>",
                        app.per.topic_sel
                    ));
                    return;
                };
                ui.horizontal(|ui| {
                    ui.heading(&topic.name);
                    ui.with_layout(
                        egui::Layout::right_to_left(egui::Align::Center),
                        |ui| match app.temp.state {
                            UiState::EditTopicDesc => {
                                if ui
                                    .button(ph::STOP_CIRCLE)
                                    .on_hover_text("Stop editing")
                                    .clicked()
                                {
                                    app.temp.state = UiState::Normal;
                                }
                            }
                            _ => {
                                if ui
                                    .button(egui_phosphor::regular::PENCIL)
                                    .on_hover_text("Edit description")
                                    .clicked()
                                {
                                    app.temp.state = UiState::EditTopicDesc;
                                }
                            }
                        },
                    );
                });
                match app.temp.state {
                    UiState::EditTopicDesc => {
                        ui.text_edit_multiline(&mut topic.desc);
                    }
                    _ => {
                        if !topic.desc.is_empty() {
                            ui.label(&topic.desc);
                        }
                    }
                }
                ui.separator();
                tasks_list_ui(ui, &mut app.temp, topic, &app.per.topic_sel);
                if let Some(sel) = topic.task_sel
                    && let Some(en) = topic.entries.get_mut(sel)
                {
                    ui.separator();
                    task_ui(en, &mut app.temp, ui, cp_avail_width);
                }
            } else {
                ui.heading("Select a topic on the left, or create one!");
            }
        });
    });
}

fn tasks_list_ui(
    ui: &mut egui::Ui,
    app_temp: &mut TodoAppTemp,
    topic: &mut Topic,
    topic_sel: &[usize],
) {
    ScrollArea::vertical()
        .auto_shrink([false; 2])
        .id_salt("tasks_scroll")
        .max_height(200.0)
        .show(ui, |ui| {
            for (i, entry) in topic.entries.iter_mut().enumerate() {
                ui.horizontal(|ui| {
                    match entry.kind {
                        EntryKind::Task => {
                            let re = ui.checkbox(&mut entry.done, "");
                            if re.changed() {
                                app_temp.per_dirty = true;
                            }
                        }
                        EntryKind::Info => {
                            ui.label("ℹ");
                        }
                    }
                    let mut text = egui::RichText::new(&entry.title);
                    if entry.done {
                        text = text.strikethrough();
                    }
                    match &app_temp.state {
                        UiState::RenameTask {
                            task_idx,
                            topic_idx,
                        } if topic_idx == topic_sel && i == *task_idx => {
                            if ui.text_edit_singleline(&mut entry.title).lost_focus() {
                                app_temp.state = UiState::Normal;
                            }
                        }
                        _ => {
                            let re = ui.selectable_label(topic.task_sel == Some(i), text);
                            if re.clicked() {
                                topic.task_sel = Some(i);
                            }
                            if re.double_clicked() {
                                app_temp.state = UiState::RenameTask {
                                    topic_idx: topic_sel.to_vec(),
                                    task_idx: topic.task_sel.unwrap(),
                                };
                            }
                        }
                    }
                });
            }
        });
    ui.separator();
    ui.horizontal(|ui| match &mut app_temp.state {
        UiState::AddTask(name) => {
            let clicked = ui.button(ph::CHECK_FAT).clicked();
            if ui.button(ph::X_CIRCLE).clicked()
                || ui.input(|inp| inp.key_pressed(egui::Key::Escape))
            {
                app_temp.state = UiState::Normal;
            } else {
                ui.text_edit_singleline(name).request_focus();
                if clicked || ui.input(|inp| inp.key_pressed(egui::Key::Enter)) {
                    topic.entries.insert(
                        topic.task_sel.map(|idx| idx + 1).unwrap_or(0),
                        Entry {
                            title: name.take(),
                            desc: String::new(),
                            done: false,
                            attachments: Vec::new(),
                            kind: EntryKind::Task,
                        },
                    );
                    app_temp.state = UiState::Normal;
                    match &mut topic.task_sel {
                        Some(sel) => {
                            if *sel + 1 < topic.entries.len() {
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
                app_temp.state = UiState::add_task();
            }
            if ui.button(ph::TRASH).clicked() {
                if let Some(task_sel) = topic.task_sel {
                    topic.entries.remove(task_sel);
                    if topic.entries.is_empty() {
                        topic.task_sel = None;
                    } else {
                        topic.task_sel = Some(task_sel.clamp(0, topic.entries.len() - 1));
                    }
                }
            }
            if let Some(task_sel) = topic.task_sel {
                if ui
                    .add_enabled(task_sel > 0, egui::Button::new(ph::ARROW_FAT_UP))
                    .clicked()
                {
                    topic.entries.swap(task_sel, task_sel - 1);
                    topic.task_sel = Some(task_sel - 1);
                }
                if ui
                    .add_enabled(
                        task_sel < topic.entries.len() - 1,
                        egui::Button::new(ph::ARROW_FAT_DOWN),
                    )
                    .clicked()
                {
                    topic.entries.swap(task_sel, task_sel + 1);
                    topic.task_sel = Some(task_sel + 1);
                }
                if ui
                    .button(ph::SORT_DESCENDING)
                    .on_hover_text("Auto sort")
                    .clicked()
                {
                    topic
                        .entries
                        .sort_by(|a, b| a.done.cmp(&b.done).then_with(|| a.title.cmp(&b.title)));
                }
                if ui
                    .button("⬈ Move")
                    .on_hover_text("Move into another topic")
                    .clicked()
                {
                    app_temp.state = UiState::MoveTaskIntoTopic(topic.entries.remove(task_sel));
                    topic.task_sel = None;
                }
                let Some(entry) = topic.entries.get_mut(task_sel) else {
                    ui.label("<error getting entry>");
                    return;
                };
                egui::ComboBox::new("kind_combo", "Kind")
                    .selected_text(entry.kind.label())
                    .show_ui(ui, |ui| {
                        ui.selectable_value(
                            &mut entry.kind,
                            EntryKind::Task,
                            EntryKind::Task.label(),
                        );
                        ui.selectable_value(
                            &mut entry.kind,
                            EntryKind::Info,
                            EntryKind::Info.label(),
                        );
                    });
            }
        }
    });
}

impl EntryKind {
    fn label(&self) -> &'static str {
        match self {
            EntryKind::Task => "☑ Task",
            EntryKind::Info => "ℹ Info",
        }
    }
}

/// UI for details about an individual task
fn task_ui(entry: &mut Entry, app_temp: &mut TodoAppTemp, ui: &mut egui::Ui, cp_avail_width: f32) {
    ui.horizontal(|ui| {
        ui.heading(&entry.title);
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.checkbox(&mut app_temp.view_task_as_markdown, "Markdown")
                .on_hover_text("View as markdown");
        });
    });
    egui::ScrollArea::vertical().show(ui, |ui| {
        if app_temp.view_task_as_markdown {
            // TODO: We might want a less expensive way to check for changes
            let prev = entry.desc.clone();
            CommonMarkViewer::new().show_mut(ui, &mut app_temp.cm_cache, &mut entry.desc);
            if entry.desc != prev {
                app_temp.per_dirty = true;
            }
        } else {
            let te = egui::TextEdit::multiline(&mut entry.desc)
                .code_editor()
                .desired_width(cp_avail_width);
            let re = ui.add(te);
            if re.changed() {
                app_temp.per_dirty = true;
            }
        }
        for attachment in &entry.attachments {
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
                                error_msgbox(
                                    &format!("Failed to create tmp dir: {}", e),
                                    &mut app_temp.modal,
                                );
                                dir_exists = false;
                            }
                        }
                    }
                    if dir_exists {
                        match std::fs::write(&path, &attachment.data) {
                            Ok(_) => {
                                if let Err(e) = open::that(path) {
                                    error_msgbox(
                                        &format!("Failed to open file: {}", e),
                                        &mut app_temp.modal,
                                    )
                                }
                            }
                            Err(e) => error_msgbox(
                                &format!("Failed to save file: {}", e),
                                &mut app_temp.modal,
                            ),
                        }
                    }
                }
            });
        }
        ui.separator();
        if ui.button("Attach files").clicked() {
            app_temp.file_dialog.pick_multiple();
        }
        if let Some(paths) = app_temp.file_dialog.take_picked_multiple() {
            for path in paths {
                if let Some(filename) = path.file_name() {
                    let data = std::fs::read(&path).unwrap();
                    entry.attachments.push(Attachment {
                        filename: filename.into(),
                        data,
                    })
                } else {
                    error_msgbox(
                        &format!("Could not determine filename for file {:?}", path),
                        &mut app_temp.modal,
                    );
                }
            }
        }
    });
}

pub fn error_msgbox(msg: &str, modal: &mut Option<ModalPayload>) {
    *modal = Some(ModalPayload::ErrorMsg(msg.to_string()));
}
