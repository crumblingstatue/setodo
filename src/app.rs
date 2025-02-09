use {
    crate::{
        cmd::Cmd,
        data::{Entry, Topic},
        tree,
        ui::{central_panel_ui, tree_view_ui},
    },
    eframe::{
        Frame,
        egui::{self, FontDefinitions, FontFamily},
    },
    egui_commonmark::CommonMarkCache,
    egui_file_dialog::FileDialog,
    egui_fontcfg::{CustomFontPaths, FontCfgUi},
    rmp_serde::Serializer,
    serde::{Deserialize, Serialize},
    std::{
        collections::BTreeMap,
        error::Error,
        fs::File,
        path::{Path, PathBuf},
    },
};

#[derive(Default, Serialize, Deserialize)]
pub struct TodoAppPersistent {
    pub topic_sel: Vec<usize>,
    pub topics: Vec<Topic>,
    #[serde(default)]
    pub stored_font_data: Option<StoredFontData>,
}

impl TodoAppPersistent {
    fn load(data_file_path: &Path) -> Result<Self, Box<dyn Error>> {
        let file = File::open(data_file_path)?;
        let dec = zstd::stream::read::Decoder::new(file)?;
        Ok(rmp_serde::from_read(dec)?)
    }
}

pub struct TodoApp {
    pub per: TodoAppPersistent,
    pub temp: TodoAppTemp,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct StoredFontData {
    pub families: BTreeMap<FontFamily, Vec<String>>,
    pub custom: CustomFontPaths,
}

/// Transient data, not saved during serialization
pub struct TodoAppTemp {
    pub state: UiState,
    pub font_defs_ui: FontCfgUi,
    /// Copy of FontDefs for editing through font config UI
    pub font_defs_edit_copy: FontDefinitions,
    /// Copy of CustomFonts for editing through font config UI
    pub custom_edit_copy: CustomFontPaths,
    pub cm_cache: CommonMarkCache,
    pub view_task_as_markdown: bool,
    pub find_string: String,
    /// If true, pressing Esc won't hide the window like it usually does
    pub esc_was_used: bool,
    /// The persistent data has been modified since the last save
    pub per_dirty: bool,
    /// Path to the data file we're reading from / writing to
    pub data_file_path: PathBuf,
    pub file_dialog: FileDialog,
    pub action_flags: ActionFlags,
    pub cmd: Vec<Cmd>,
    pub modal: Option<ModalPayload>,
}

pub enum ModalPayload {
    ErrorMsg(String),
}

#[derive(Default)]
pub struct ActionFlags {
    pub expand_all: bool,
    pub collapse_all: bool,
}
impl ActionFlags {
    fn clear(&mut self) {
        *self = Self::default();
    }
}

impl TodoAppTemp {
    fn new(data_file_path: PathBuf) -> Self {
        Self {
            state: UiState::Normal,
            font_defs_ui: Default::default(),
            font_defs_edit_copy: FontDefinitions::default(),
            custom_edit_copy: Default::default(),
            cm_cache: CommonMarkCache::default(),
            view_task_as_markdown: false,
            find_string: String::new(),
            esc_was_used: false,
            per_dirty: false,
            data_file_path,
            file_dialog: FileDialog::new(),
            action_flags: Default::default(),
            cmd: Vec::new(),
            modal: None,
        }
    }
}

pub enum UiState {
    Normal,
    AddSubtopic {
        name: String,
        parent_idx: Vec<usize>,
    },
    AddTask(String),
    MoveTopicInto {
        src_idx: Vec<usize>,
    },
    MoveTaskIntoTopic(Entry),
    RenameTopic {
        idx: Vec<usize>,
    },
    RenameTask {
        topic_idx: Vec<usize>,
        task_idx: usize,
    },
    FontCfg,
    EditTopicDesc,
}

impl UiState {
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

pub fn default_data_file_path() -> PathBuf {
    dirs_sys::home_dir().unwrap().join(".setodo.dat")
}

impl TodoApp {
    pub fn new(data_file_path: PathBuf) -> Self {
        Self {
            per: TodoAppPersistent::default(),
            temp: TodoAppTemp::new(data_file_path),
        }
    }
    pub fn load(data_file_path: PathBuf) -> Result<Self, Box<dyn Error>> {
        Ok(Self {
            per: TodoAppPersistent::load(&data_file_path)?,
            temp: TodoAppTemp::new(data_file_path),
        })
    }
    pub fn save_persistent(&mut self) -> Result<(), Box<dyn Error>> {
        let file = File::create(&self.temp.data_file_path)?;
        let mut enc = zstd::stream::write::Encoder::new(file, zstd::DEFAULT_COMPRESSION_LEVEL)?;
        self.per.serialize(&mut Serializer::new(&mut enc))?;
        enc.finish()?;
        self.temp.per_dirty = false;
        Ok(())
    }
    pub fn reload_persistent(&mut self) -> Result<(), Box<dyn Error>> {
        let per = TodoAppPersistent::load(&self.temp.data_file_path)?;
        self.per = per;
        self.temp.per_dirty = false;
        Ok(())
    }
}

impl eframe::App for TodoApp {
    fn on_exit(&mut self, _ctx: Option<&eframe::glow::Context>) {
        self.save_persistent().unwrap();
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut Frame) {
        let (ctrl, btn_r, btn_s) = ctx.input(|inp| {
            (
                inp.modifiers.ctrl,
                inp.key_pressed(egui::Key::R),
                inp.key_pressed(egui::Key::S),
            )
        });
        if ctrl && btn_s {
            if let Err(e) = self.save_persistent() {
                eprintln!("Error when saving: {e}");
            }
        }
        if ctrl && btn_r {
            if let Err(e) = self.reload_persistent() {
                eprintln!("Error reloading: {e}");
            }
        }
        egui::SidePanel::left("tree_view").show(ctx, |ui| tree_view_ui(ui, self));
        egui::CentralPanel::default().show(ctx, |ui| central_panel_ui(ui, self));
        self.temp.file_dialog.update(ctx);
        if ctx.input(|inp| inp.key_pressed(egui::Key::Escape)) && !self.temp.esc_was_used {
            ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
            if let Err(e) = self.save_persistent() {
                eprintln!("Autosave error: {e}");
            }
        }
        self.temp.esc_was_used = false;
        self.temp.action_flags.clear();
        self.temp.cmd.retain(|cmd| {
            let mut retain = true;
            match cmd {
                Cmd::RemoveTopic { idx } => {
                    tree::remove(&mut self.per.topics, idx);
                    self.temp.per_dirty = true;
                    retain = false;
                }
                Cmd::FocusTextEdit => {}
            }
            retain
        });
        if let Some(payload) = &self.temp.modal {
            let mut close = false;
            egui::Modal::new("modal_popup".into()).show(ctx, |ui| match payload {
                ModalPayload::ErrorMsg(msg) => {
                    ui.heading("Error");
                    ui.label(msg);
                    if ui.button("Close").clicked() {
                        close = true;
                    }
                }
            });
            if close {
                self.temp.modal = None;
            }
        }
    }
}

pub fn move_task_into_topic(
    topics: &mut [Topic],
    task: Entry,
    topic_sel: &[usize],
) -> Result<(), ()> {
    let Some(topic) = tree::get_mut(topics, topic_sel) else {
        return Err(());
    };
    topic.entries.push(task);
    Ok(())
}
