use {
    crate::{
        data::{Entry, Topic},
        ui::{central_panel_ui, tree_view_ui},
    },
    eframe::{
        egui::{self, FontDefinitions, FontFamily},
        Frame,
    },
    egui_commonmark::CommonMarkCache,
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
    dirs::home_dir().unwrap().join(".setodo.dat")
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
        if ctx.input(|inp| inp.key_pressed(egui::Key::Escape)) && !self.temp.esc_was_used {
            ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
            if let Err(e) = self.save_persistent() {
                eprintln!("Autosave error: {e}");
            }
        }
        self.temp.esc_was_used = false;
    }
}

pub fn move_task_into_topic(
    topics: &mut [Topic],
    task: Entry,
    topic_sel: &[usize],
) -> Result<(), ()> {
    let Some(topic) = get_topic_mut(topics, topic_sel) else {
        return Err(());
    };
    topic.entries.push(task);
    Ok(())
}

pub fn move_topic_precondition(src_idx: &[usize], dst_idx: &[usize]) -> bool {
    !(dst_idx.len() >= src_idx.len() && src_idx == &dst_idx[..src_idx.len()])
}

pub fn move_topic(topics: &mut Vec<Topic>, src_idx: &[usize], dst_idx: &[usize]) {
    if !move_topic_precondition(src_idx, dst_idx) {
        return;
    }
    if let Some(topic) = remove_topic(topics, src_idx) {
        insert_topic(topics, dst_idx, topic);
    }
}

pub fn get_topic_mut<'t>(mut topics: &'t mut [Topic], indices: &[usize]) -> Option<&'t mut Topic> {
    for i in 0..indices.len() {
        let idx = *indices.get(i)?;
        if i == indices.len() - 1 {
            return topics.get_mut(idx);
        } else {
            topics = &mut topics.get_mut(idx)?.children;
        }
    }
    None
}

pub fn remove_topic(mut topics: &mut Vec<Topic>, indices: &[usize]) -> Option<Topic> {
    let mut index = None;
    for i in 0..indices.len() {
        let idx = indices[i];
        index = Some(idx);
        if i == indices.len() - 1 {
            break;
        } else {
            topics = &mut topics.get_mut(idx)?.children;
        }
    }
    index.map(|idx| topics.remove(idx))
}

pub fn insert_topic(mut topics: &mut Vec<Topic>, indices: &[usize], topic: Topic) {
    for &idx in indices {
        topics = &mut topics[idx].children;
    }
    topics.push(topic);
}
