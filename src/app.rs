use {
    crate::{
        data::{Task, Topic},
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
    std::{collections::BTreeMap, error::Error, fs::File, path::PathBuf},
};

#[derive(Default, Serialize, Deserialize)]
pub struct TodoAppPersistent {
    pub topic_sel: Vec<usize>,
    pub topics: Vec<Topic>,
    #[serde(default)]
    pub stored_font_data: Option<StoredFontData>,
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
}

impl TodoAppTemp {
    fn new() -> Self {
        Self {
            state: UiState::Normal,
            font_defs_ui: Default::default(),
            font_defs_edit_copy: FontDefinitions::default(),
            custom_edit_copy: Default::default(),
            cm_cache: CommonMarkCache::default(),
            view_task_as_markdown: false,
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
    FontCfg,
    EditTopicDesc,
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
    pub fn new() -> Self {
        Self {
            per: TodoAppPersistent::default(),
            temp: TodoAppTemp::new(),
        }
    }
    pub fn load() -> Result<Self, Box<dyn Error>> {
        let file = File::open(file_name())?;
        let dec = zstd::stream::read::Decoder::new(file)?;
        let per: TodoAppPersistent = rmp_serde::from_read(dec)?;
        Ok(Self {
            per,
            temp: TodoAppTemp::new(),
        })
    }
    fn save(&self) -> Result<(), Box<dyn Error>> {
        let file = File::create(file_name())?;
        let mut enc = zstd::stream::write::Encoder::new(file, zstd::DEFAULT_COMPRESSION_LEVEL)?;
        self.per.serialize(&mut Serializer::new(&mut enc))?;
        enc.finish()?;
        Ok(())
    }
}

impl eframe::App for TodoApp {
    fn on_exit(&mut self, _ctx: Option<&eframe::glow::Context>) {
        TodoApp::save(self).unwrap();
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut Frame) {
        egui::SidePanel::left("tree_view").show(ctx, |ui| tree_view_ui(ui, self));
        egui::CentralPanel::default().show(ctx, |ui| central_panel_ui(ui, self));
        if ctx.input(|inp| inp.key_pressed(egui::Key::Escape)) {
            ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
        }
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
