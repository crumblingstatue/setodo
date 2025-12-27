use {
    crate::{
        app::{ConfirmAction, ModalPayload, StoredFontData, TodoApp, TodoAppTemp, UiState},
        cmd::Cmd,
        data::{Attachment, Entry, EntryKind, Topic},
        tree,
    },
    eframe::egui::{self, KeyboardShortcut, TextBuffer as _},
    egui_commonmark::CommonMarkViewer,
    egui_fontcfg::FontDefsUiMsg,
    egui_phosphor::regular as ph,
};

pub fn ui(ui: &mut egui::Ui, app: &mut TodoApp) {
    if matches!(app.temp.state, UiState::FontCfg) {
        font_defs_ui(ui, app);
        return;
    }
    let cp_avail_height = ui.available_height();
    ui.horizontal(|ui| {
        ui.set_min_height(cp_avail_height);
        let cp_avail_width = ui.available_width();
        ui.vertical(|ui| {
            if app.per.topic_sel.is_empty() {
                ui.heading("Select a topic on the left, or create one!");
            } else {
                let Some(topic) = tree::get_mut(&mut app.per.topics, &app.per.topic_sel) else {
                    ui.label(format!(
                        "<error getting topic. index: {:?}>",
                        app.per.topic_sel
                    ));
                    return;
                };
                ui.horizontal(|ui| {
                    ui.heading(&topic.name);
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if let UiState::EditTopicDesc = app.temp.state {
                            if ui
                                .button(ph::STOP_CIRCLE)
                                .on_hover_text("Stop editing")
                                .clicked()
                            {
                                app.temp.state = UiState::Normal;
                            }
                        } else {
                            if ui
                                .button(ph::TRASH)
                                .on_hover_text("Clear topic entries")
                                .clicked()
                            {
                                app.temp.confirm_action = Some(ConfirmAction::ClearTopicEntries);
                            }
                            if ui
                                .button(egui_phosphor::regular::PENCIL)
                                .on_hover_text("Edit description")
                                .clicked()
                            {
                                app.temp.state = UiState::EditTopicDesc;
                            }
                            if ui
                                .button(egui_phosphor::regular::CURSOR_TEXT)
                                .on_hover_text("Edit title")
                                .clicked()
                            {
                                app.temp.state = UiState::RenameTopic {
                                    idx: app.per.topic_sel.clone(),
                                };
                                app.temp.cmd.push(Cmd::FocusTextEdit);
                            }
                        }
                    });
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
                    let cmd = task_ui(en, &mut app.temp, ui, cp_avail_width);
                    if let Some(cmd) = cmd {
                        match cmd {
                            TaskUiCmd::GotoEntry { title } => {
                                if let Some(pos) =
                                    topic.entries.iter().position(|en| en.title == title)
                                {
                                    topic.task_sel = Some(pos);
                                }
                            }
                        }
                    }
                }
            }
        });
    });
}

fn font_defs_ui(ui: &mut egui::Ui, app: &mut TodoApp) {
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
}

fn tasks_list_ui(
    ui: &mut egui::Ui,
    app_temp: &mut TodoAppTemp,
    topic: &mut Topic,
    topic_sel: &[usize],
) {
    egui::ScrollArea::vertical()
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
    ui.horizontal(|ui| {
        if let UiState::AddTask(name) = &mut app_temp.state {
            let clicked = ui.button(ph::CHECK_FAT).clicked();
            if ui.button(ph::X_CIRCLE).clicked()
                || ui.input(|inp| inp.key_pressed(egui::Key::Escape))
            {
                app_temp.state = UiState::Normal;
            } else {
                ui.text_edit_singleline(name).request_focus();
                if clicked || ui.input(|inp| inp.key_pressed(egui::Key::Enter)) {
                    topic.entries.insert(
                        topic.task_sel.map_or(0, |idx| idx + 1),
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
        } else {
            tasks_list_bottom_bar_default_ui(app_temp, topic, ui);
        }
    });
}

const ADD_SHORTCUT: KeyboardShortcut = KeyboardShortcut::new(egui::Modifiers::CTRL, egui::Key::N);
const DEL_SHORTCUT: KeyboardShortcut =
    KeyboardShortcut::new(egui::Modifiers::CTRL, egui::Key::Delete);

fn tasks_list_bottom_bar_default_ui(
    app_temp: &mut TodoAppTemp,
    topic: &mut Topic,
    ui: &mut egui::Ui,
) {
    if ui
        .button(ph::FILE_PLUS)
        .on_hover_text(format!(
            "Add entry ({})",
            ui.ctx().format_shortcut(&ADD_SHORTCUT)
        ))
        .clicked()
        || ui.input_mut(|inp| inp.consume_shortcut(&ADD_SHORTCUT))
    {
        app_temp.state = UiState::add_task();
    }
    if (ui
        .button(ph::TRASH)
        .on_hover_text(format!(
            "Delete selected entry ({})",
            ui.ctx().format_shortcut(&DEL_SHORTCUT)
        ))
        .clicked()
        || ui.input_mut(|inp| inp.consume_shortcut(&DEL_SHORTCUT)))
        && let Some(task_sel) = topic.task_sel
    {
        topic.entries.remove(task_sel);
        if topic.entries.is_empty() {
            topic.task_sel = None;
        } else {
            topic.task_sel = Some(task_sel.clamp(0, topic.entries.len() - 1));
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
                ui.selectable_value(&mut entry.kind, EntryKind::Task, EntryKind::Task.label());
                ui.selectable_value(&mut entry.kind, EntryKind::Info, EntryKind::Info.label());
            });
    }
}

impl EntryKind {
    const fn label(&self) -> &'static str {
        match self {
            Self::Task => "☑ Task",
            Self::Info => "ℹ Info",
        }
    }
}

enum TaskUiCmd {
    GotoEntry { title: String },
}

/// UI for details about an individual task
#[must_use]
fn task_ui(
    entry: &mut Entry,
    app_temp: &mut TodoAppTemp,
    ui: &mut egui::Ui,
    cp_avail_width: f32,
) -> Option<TaskUiCmd> {
    let mut out_cmd = None;
    ui.horizontal(|ui| {
        ui.heading(&entry.title);
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.checkbox(&mut app_temp.view_task_as_markdown, "Markdown")
                .on_hover_text("View as markdown");
        });
    });
    egui::ScrollArea::vertical().show(ui, |ui| {
        ui.style_mut().url_in_tooltip = true;
        if app_temp.view_task_as_markdown {
            // TODO: We might want a less expensive way to check for changes
            let prev = entry.desc.clone();
            CommonMarkViewer::new().show_mut(ui, &mut app_temp.cm_cache, &mut entry.desc);
            ui.output_mut(|out| {
                out.commands.retain(|cmd| {
                    let mut retain = true;
                    if let egui::OutputCommand::OpenUrl(url) = cmd
                        && let Some(en_title) = url.url.strip_prefix("entry://")
                    {
                        out_cmd = Some(TaskUiCmd::GotoEntry {
                            title: en_title.to_owned(),
                        });
                        retain = false;
                    }
                    retain
                });
            });
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
        task_attachments_ui(entry, app_temp, ui);
    });
    out_cmd
}

fn task_attachments_ui(entry: &mut Entry, app_temp: &mut TodoAppTemp, ui: &mut egui::Ui) {
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
                        Ok(()) => {
                            dir_exists = true;
                        }
                        Err(e) => {
                            error_msgbox(
                                &format!("Failed to create tmp dir: {e}"),
                                &mut app_temp.modal,
                            );
                            dir_exists = false;
                        }
                    }
                }
                if dir_exists {
                    match std::fs::write(&path, &attachment.data) {
                        Ok(()) => {
                            if let Err(e) = open::that(path) {
                                error_msgbox(
                                    &format!("Failed to open file: {e}"),
                                    &mut app_temp.modal,
                                );
                            }
                        }
                        Err(e) => {
                            error_msgbox(&format!("Failed to save file: {e}"), &mut app_temp.modal);
                        }
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
                });
            } else {
                error_msgbox(
                    &format!("Could not determine filename for file '{}'", path.display()),
                    &mut app_temp.modal,
                );
            }
        }
    }
}

pub fn error_msgbox(msg: &str, modal: &mut Option<ModalPayload>) {
    *modal = Some(ModalPayload::ErrorMsg(msg.to_string()));
}
