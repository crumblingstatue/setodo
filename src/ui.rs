use {
    crate::{
        app::{
            get_topic_mut, move_task_into_topic, move_topic, remove_topic, StoredFontData, TodoApp,
            UiState,
        },
        data::{Attachment, Entry, EntryKind, Topic},
    },
    constcat::concat as cc,
    eframe::egui::{
        self, collapsing_header::CollapsingState, ScrollArea, TextBuffer, ViewportCommand,
    },
    egui_commonmark::CommonMarkViewer,
    egui_fontcfg::FontDefsUiMsg,
    egui_phosphor::regular as ph,
};

pub fn tree_view_ui(ui: &mut egui::Ui, app: &mut TodoApp) {
    let esc_pressed = ui.input(|inp| inp.key_pressed(egui::Key::Escape));
    ScrollArea::vertical()
        .id_source("topics_scroll")
        .show(ui, |ui| {
            ui.vertical(|ui| {
                ui.horizontal(|ui| {
                    ui.scope(|ui| {
                        ui.spacing_mut().item_spacing = egui::vec2(0., 0.);
                        ui.heading("Topics");
                        if app.temp.per_dirty {
                            ui.label(
                                egui::RichText::new("*")
                                    .strong()
                                    .size(20.0)
                                    .color(egui::Color32::YELLOW),
                            )
                            .on_hover_text("There are unsaved changes");
                        }
                    });
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
                                match TodoApp::load() {
                                    Ok(new) => *app = new,
                                    Err(e) => eprintln!("Reload error: {e}"),
                                }
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
                        if ui.button("👁 Hide").on_hover_text("Hotkey: Esc").clicked() {
                            ui.ctx()
                                .send_viewport_cmd(egui::ViewportCommand::Visible(false));
                            if let Err(e) = app.save_persistent() {
                                eprintln!("Autosave error: {e}");
                            }
                        };
                        let re = ui.add(
                            egui::TextEdit::singleline(&mut app.temp.find_string)
                                .hint_text("🔍 Find (ctrl+F)"),
                        );
                        if re.changed() {
                            app.temp.per_dirty = true;
                        }
                        if ui.input(|inp| inp.modifiers.ctrl && inp.key_pressed(egui::Key::F)) {
                            re.request_focus();
                        }
                        if !app.temp.find_string.is_empty() && esc_pressed {
                            app.temp.esc_was_used = true;
                            app.temp.find_string.clear();
                        }
                    });
                });
                if !app.temp.find_string.is_empty() {
                    find_ui(ui, app);
                    return;
                }
                let any_clicked = topics_ui(
                    &mut app.per.topics,
                    &mut Vec::new(),
                    &mut app.per.topic_sel,
                    ui,
                    &mut app.temp.state,
                    &mut app.temp.per_dirty,
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
                                app.per.topics.push(Topic {
                                    name: name.take(),
                                    desc: String::new(),
                                    entries: Vec::new(),
                                    task_sel: None,
                                    children: Vec::new(),
                                });
                                app.temp.state = UiState::Normal;
                                // TODO: Do something more reasonable here
                                app.per.topic_sel.clear();
                                app.temp.per_dirty = true;
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
                                let topic = get_topic_mut(&mut app.per.topics, parent_idx);
                                topic.children.push(Topic {
                                    name: name.take(),
                                    desc: String::new(),
                                    entries: Vec::new(),
                                    task_sel: None,
                                    children: Vec::new(),
                                });
                                app.temp.state = UiState::Normal;
                                // TODO: Do something more reasonable here
                                app.per.topic_sel.clear();
                                app.temp.per_dirty = true;
                            }
                        }
                    }
                    UiState::MoveTopicInto { src_idx } => {
                        ui.label("Click on topic to move into!");
                        ui.label(any_clicked.to_string());
                        if any_clicked {
                            move_topic(&mut app.per.topics, src_idx, &app.per.topic_sel);
                            app.temp.state = UiState::Normal;
                        }
                    }
                    UiState::MoveTaskIntoTopic(task) => {
                        if any_clicked {
                            move_task_into_topic(
                                &mut app.per.topics,
                                std::mem::take(task),
                                &app.per.topic_sel,
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
                                    !app.per.topic_sel.is_empty(),
                                    egui::Button::new(ph::TRASH),
                                )
                                .clicked()
                                && !app.per.topic_sel.is_empty()
                            {
                                remove_topic(&mut app.per.topics, &app.per.topic_sel);
                                // TODO: Do something more reasonable
                                app.per.topic_sel.clear();
                            }
                            if let Some(topic_sel) = app.per.topic_sel.last_mut() {
                                if ui
                                    .add_enabled(
                                        *topic_sel > 0,
                                        egui::Button::new(ph::ARROW_FAT_UP),
                                    )
                                    .clicked()
                                {
                                    app.per.topics.swap(*topic_sel, *topic_sel - 1);
                                    *topic_sel -= 1;
                                }
                                if ui
                                    .add_enabled(
                                        *topic_sel < app.per.topics.len() - 1,
                                        egui::Button::new(ph::ARROW_FAT_DOWN),
                                    )
                                    .clicked()
                                {
                                    app.per.topics.swap(*topic_sel, *topic_sel + 1);
                                    *topic_sel += 1;
                                }
                                if ui.button("Add subtopic").clicked() {
                                    app.temp.state =
                                        UiState::add_subtopic(app.per.topic_sel.clone());
                                }
                                if ui.button("Move topic into").clicked() {
                                    app.temp.state =
                                        UiState::move_topic_into(app.per.topic_sel.clone());
                                }
                            }
                        });
                    }
                });
            });
        });
}

fn find_ui(ui: &mut egui::Ui, app: &mut TodoApp) {
    let find_str = &app.temp.find_string;
    for matched_topic in collect_matches(&app.per.topics, find_str) {
        let topic = get_topic_mut(&mut app.per.topics, &matched_topic.cursor);
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

fn topics_ui(
    topics: &mut [Topic],
    cursor: &mut Vec<usize>,
    topic_sel: &mut Vec<usize>,
    ui: &mut egui::Ui,
    state: &mut UiState,
    per_dirty: &mut bool,
) -> bool {
    let mut any_clicked = false;
    cursor.push(0);
    for (i, topic) in topics.iter_mut().enumerate() {
        *cursor.last_mut().unwrap() = i;
        match state {
            UiState::RenameTopic { idx } if idx == cursor => {
                let re = ui.text_edit_singleline(&mut topic.name);
                if re.lost_focus() {
                    *state = UiState::Normal;
                }
                if re.changed() {
                    *per_dirty = true;
                }
            }
            _ => {
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
                } else {
                    let id = ui.make_persistent_id("cheader").with(&topic.name);
                    CollapsingState::load_with_default_open(ui.ctx(), id, false)
                        .show_header(ui, |ui| {
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
                        })
                        .body(|ui| {
                            any_clicked |= topics_ui(
                                &mut topic.children,
                                cursor,
                                topic_sel,
                                ui,
                                state,
                                per_dirty,
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
                let topic = get_topic_mut(&mut app.per.topics, &app.per.topic_sel);
                ui.horizontal(|ui| {
                    ui.heading(&topic.name);
                    ui.with_layout(
                        egui::Layout::right_to_left(egui::Align::Center),
                        |ui| match app.temp.state {
                            UiState::EditTopicDesc => {
                                if ui
                                    .button(egui_phosphor::regular::X_CIRCLE)
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
                tasks_list_ui(ui, app);
                if let Some(task_sel) =
                    get_topic_mut(&mut app.per.topics, &app.per.topic_sel).task_sel
                {
                    ui.separator();
                    task_ui(app, task_sel, ui, cp_avail_width);
                }
            }
        });
    });
}

fn tasks_list_ui(ui: &mut egui::Ui, app: &mut TodoApp) {
    ScrollArea::vertical()
        .auto_shrink([false; 2])
        .id_source("tasks_scroll")
        .max_height(200.0)
        .show(ui, |ui| {
            let topic = get_topic_mut(&mut app.per.topics, &app.per.topic_sel);
            for (i, entry) in topic.entries.iter_mut().enumerate() {
                ui.horizontal(|ui| {
                    match entry.kind {
                        EntryKind::Task => {
                            let re = ui.checkbox(&mut entry.done, "");
                            if re.changed() {
                                app.temp.per_dirty = true;
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
                    match &app.temp.state {
                        UiState::RenameTask {
                            task_idx,
                            topic_idx,
                        } if topic_idx == &app.per.topic_sel && i == *task_idx => {
                            if ui.text_edit_singleline(&mut entry.title).lost_focus() {
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
                                    topic_idx: app.per.topic_sel.clone(),
                                    task_idx: topic.task_sel.unwrap(),
                                };
                            }
                        }
                    }
                });
            }
        });
    ui.separator();
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
                    let topic = get_topic_mut(&mut app.per.topics, &app.per.topic_sel);
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
                    app.temp.state = UiState::Normal;
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
                app.temp.state = UiState::add_task();
            }
            if ui.button(ph::TRASH).clicked() {
                if let Some(task_sel) =
                    get_topic_mut(&mut app.per.topics, &app.per.topic_sel).task_sel
                {
                    get_topic_mut(&mut app.per.topics, &app.per.topic_sel)
                        .entries
                        .remove(task_sel);
                    if get_topic_mut(&mut app.per.topics, &app.per.topic_sel)
                        .entries
                        .is_empty()
                    {
                        get_topic_mut(&mut app.per.topics, &app.per.topic_sel).task_sel = None;
                    } else {
                        get_topic_mut(&mut app.per.topics, &app.per.topic_sel).task_sel = Some(
                            task_sel.clamp(
                                0,
                                get_topic_mut(&mut app.per.topics, &app.per.topic_sel)
                                    .entries
                                    .len()
                                    - 1,
                            ),
                        );
                    }
                }
            }
            if let Some(task_sel) = get_topic_mut(&mut app.per.topics, &app.per.topic_sel).task_sel
            {
                if ui
                    .add_enabled(task_sel > 0, egui::Button::new(ph::ARROW_FAT_UP))
                    .clicked()
                {
                    get_topic_mut(&mut app.per.topics, &app.per.topic_sel)
                        .entries
                        .swap(task_sel, task_sel - 1);
                    get_topic_mut(&mut app.per.topics, &app.per.topic_sel).task_sel =
                        Some(task_sel - 1);
                }
                if ui
                    .add_enabled(
                        task_sel
                            < get_topic_mut(&mut app.per.topics, &app.per.topic_sel)
                                .entries
                                .len()
                                - 1,
                        egui::Button::new(ph::ARROW_FAT_DOWN),
                    )
                    .clicked()
                {
                    get_topic_mut(&mut app.per.topics, &app.per.topic_sel)
                        .entries
                        .swap(task_sel, task_sel + 1);
                    get_topic_mut(&mut app.per.topics, &app.per.topic_sel).task_sel =
                        Some(task_sel + 1);
                }
                if ui
                    .button(ph::SORT_DESCENDING)
                    .on_hover_text("Auto sort")
                    .clicked()
                {
                    get_topic_mut(&mut app.per.topics, &app.per.topic_sel)
                        .entries
                        .sort_by(|a, b| a.done.cmp(&b.done).then_with(|| a.title.cmp(&b.title)));
                }
                if ui
                    .button("⬈ Move")
                    .on_hover_text("Move into another topic")
                    .clicked()
                {
                    let topic = get_topic_mut(&mut app.per.topics, &app.per.topic_sel);
                    app.temp.state = UiState::MoveTaskIntoTopic(topic.entries.remove(task_sel));
                    get_topic_mut(&mut app.per.topics, &app.per.topic_sel).task_sel = None;
                }
                let entry =
                    &mut get_topic_mut(&mut app.per.topics, &app.per.topic_sel).entries[task_sel];
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
fn task_ui(app: &mut TodoApp, task_sel: usize, ui: &mut egui::Ui, cp_avail_width: f32) {
    let task = &mut get_topic_mut(&mut app.per.topics, &app.per.topic_sel).entries[task_sel];
    ui.horizontal(|ui| {
        ui.heading(&task.title);
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.checkbox(&mut app.temp.view_task_as_markdown, "Markdown")
                .on_hover_text("View as markdown");
        });
    });
    if app.temp.view_task_as_markdown {
        // TODO: We might want a less expensive way to check for changes
        let prev = task.desc.clone();
        CommonMarkViewer::new("cm_viewer").show_mut(ui, &mut app.temp.cm_cache, &mut task.desc);
        if task.desc != prev {
            app.temp.per_dirty = true;
        }
    } else {
        let te = egui::TextEdit::multiline(&mut task.desc)
            .code_editor()
            .desired_width(cp_avail_width);
        let re = ui.add(te);
        if re.changed() {
            app.temp.per_dirty = true;
        }
    }
    for attachment in
        &get_topic_mut(&mut app.per.topics, &app.per.topic_sel).entries[task_sel].attachments
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
                    get_topic_mut(&mut app.per.topics, &app.per.topic_sel).entries[task_sel]
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
