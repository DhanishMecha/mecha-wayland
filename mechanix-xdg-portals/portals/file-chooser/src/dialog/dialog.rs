use crate::backend::{ FileChooserOutcome, FileChooserResponse, RequestHandle };
use assets::BakedFont;
use interactivity::InteractivityState;
use std::path::PathBuf;
use taffy::prelude::*;
use ui::widgets::{ Div, Text };
use ui::{ Point, RenderCommand, Widget, WidgetList, WidgetTree };
use utils::Color;

use super::widgets::{ FileEntry, FileRow, FileRows, Footer };
use super::ChooserOptions;
use super::PENDING_DIALOG;
use portal_core::atlas;

// Helper to convert Vec<u8> to PathBuf safely
fn path_from_bytes(bytes: &[u8]) -> Option<PathBuf> {
    use std::os::unix::ffi::OsStrExt;
    if bytes.is_empty() {
        return None;
    }
    let bytes = if bytes.last() == Some(&0) { &bytes[..bytes.len() - 1] } else { bytes };
    Some(std::path::Path::new(std::ffi::OsStr::from_bytes(bytes)).to_path_buf())
}

// --- Widget tree type aliases ------------------------------------------------
pub type HeaderDiv = Div<(Text, Text)>;
pub type FileListDiv = Div<FileRows>;
pub type ModalDiv = Div<(HeaderDiv, FileListDiv, Footer)>;
pub type RootDiv = Div<(ModalDiv,)>;

// Struct representing the file chooser dialog interface (rendered inside a top-level XDG window).
pub struct FileChooserUi {
    handle: RequestHandle,
    options: ChooserOptions,
    current_dir: PathBuf,
    selected_paths: Vec<PathBuf>,
    root: RootDiv,
    choose_rect: utils::Rect,
    cancel_rect: utils::Rect,
    row_rects: Vec<(u64, utils::Rect, PathBuf)>,
    choose_id: Option<u64>,
    cancel_id: Option<u64>,
}

impl FileChooserUi {
    pub fn new(handle: RequestHandle, options: ChooserOptions) -> Self {
        let initial_dir = match &options {
            ChooserOptions::OpenFile(opt) => {
                opt.current_folder.as_deref().and_then(path_from_bytes)
            }
            ChooserOptions::SaveFile(opt) => {
                opt.current_folder.as_deref().and_then(path_from_bytes)
            }
            ChooserOptions::SaveFiles(opt) => {
                opt.current_folder.as_deref().and_then(path_from_bytes)
            }
        };
        let current_dir = initial_dir
            .filter(|p| p.exists())
            .unwrap_or_else(|| {
                std::env
                    ::var("HOME")
                    .map(PathBuf::from)
                    .unwrap_or_else(|_| PathBuf::from("/"))
            });

        let root = make_root(&atlas::UI_FONT_INTER_24, &atlas::UI_FONT_INTER_16, &handle, &options);

        Self {
            handle,
            options,
            current_dir,
            selected_paths: Vec::new(),
            root,
            choose_rect: utils::Rect::ZERO,
            cancel_rect: utils::Rect::ZERO,
            row_rects: Vec::new(),
            choose_id: None,
            cancel_id: None,
        }
    }

    fn load_directory(&mut self, tree: &mut WidgetTree) {
        let mut entries = Vec::new();

        // 1. Add parent directory option if not at the filesystem root
        if let Some(parent) = self.current_dir.parent() {
            println!("[ui] Adding parent directory: {:?}", parent.to_path_buf());
            entries.push(FileEntry {
                name: "..".to_string(),
                path: parent.to_path_buf(),
                is_dir: true,
            });
        }

        // 2. Read directory entries
        if let Ok(read_dir) = std::fs::read_dir(&self.current_dir) {
            let mut file_entries = Vec::new();
            for entry in read_dir.flatten() {
                let name = entry.file_name().to_string_lossy().into_owned();
                if name.starts_with('.') {
                    continue;
                }
                let file_type = entry.file_type();
                let is_dir = file_type.map(|t| t.is_dir()).unwrap_or(false);
                let name_lower = name.to_lowercase();
                file_entries.push((
                    FileEntry {
                        name,
                        path: entry.path(),
                        is_dir,
                    },
                    name_lower,
                ));
            }

            // Sort: directories first, then files alphabetically (case-insensitive)
            file_entries.sort_by(|a, b| {
                if a.0.is_dir == b.0.is_dir {
                    a.1.cmp(&b.1)
                } else if a.0.is_dir {
                    std::cmp::Ordering::Less
                } else {
                    std::cmp::Ordering::Greater
                }
            });

            entries.extend(file_entries.into_iter().map(|(entry, _)| entry));
        }

        // 3. Update path in header
        let dir_str = self.current_dir.to_string_lossy().into_owned();
        let display_dir = if dir_str.len() > 50 {
            format!("...{}", &dir_str[dir_str.len() - 47..])
        } else {
            dir_str
        };
        self.root.children.0.children.0.children.1.set_text(tree, format!("Path: {}", display_dir));

        // 4. Update the 10 static rows
        let show_more = entries.len() > 10;
        let limit = if show_more { 9 } else { entries.len() };
        let rows = &mut self.root.children.0.children.1.children.0;

        for i in 0..10 {
            if i < limit {
                let entry = entries.get(i).cloned();
                let is_selected = entry
                    .as_ref()
                    .map_or(false, |e| self.selected_paths.contains(&e.path));
                rows[i].update(tree, entry, is_selected);
            } else if i == 9 && show_more {
                let more_count = entries.len() - 9;
                let entry = FileEntry {
                    name: format!("+ {} more items...", more_count),
                    path: PathBuf::new(),
                    is_dir: false,
                };
                rows[i].update(tree, Some(entry), false);
            } else {
                rows[i].update(tree, None, false);
            }
        }
    }
}

impl WidgetList for FileChooserUi {
    fn build_children(&mut self, tree: &mut WidgetTree) -> Vec<taffy::NodeId> {
        let child_ids = vec![self.root.build_tree(tree)];
        self.cancel_id = Some(self.root.children.0.children.2.div.children.0.node_id().into());
        self.choose_id = Some(self.root.children.0.children.2.div.children.1.node_id().into());
        self.load_directory(tree);
        child_ids
    }

    fn render_children(&mut self, tree: &WidgetTree, parent_abs: Point) -> Vec<RenderCommand> {
        let commands = self.root.render_children(tree, parent_abs);
        let choose_id = self.choose_id.unwrap_or(0);
        let cancel_id = self.cancel_id.unwrap_or(0);

        self.row_rects.clear();
        let rows = &self.root.children.0.children.1.children.0;
        for cmd in &commands {
            if let RenderCommand::RegisterHitArea { id, rect } = cmd {
                if *id == choose_id {
                    self.choose_rect = *rect;
                } else if *id == cancel_id {
                    self.cancel_rect = *rect;
                } else if let Some(row) = rows.iter().find(|r| u64::from(r.node_id()) == *id) {
                    if let Some(path) = &row.path {
                        self.row_rects.push((*id, *rect, path.clone()));
                    }
                }
            }
        }
        commands
    }

    fn on_event(&mut self, interactivity: &InteractivityState, tree: &mut WidgetTree) -> bool {
        // 1. Check Choose button
        if self.choose_rect != utils::Rect::ZERO && interactivity.is_clicked(self.choose_rect) {
            let selected_uris = match &self.options {
                ChooserOptions::OpenFile(opt) => {
                    if opt.directory.unwrap_or(false) {
                        vec![format!("file://{}", self.current_dir.to_string_lossy())]
                    } else {
                        self.selected_paths
                            .iter()
                            .map(|p| format!("file://{}", p.to_string_lossy()))
                            .collect()
                    }
                }
                ChooserOptions::SaveFile(opt) => {
                    if let Some(sel) = self.selected_paths.first() {
                        vec![format!("file://{}", sel.to_string_lossy())]
                    } else {
                        let mut path = self.current_dir.clone();
                        if let Some(name) = &opt.current_name {
                            path.push(name);
                        } else if let Some(file_bytes) = &opt.current_file {
                            if let Some(file_path) = path_from_bytes(file_bytes) {
                                path = file_path;
                            }
                        } else {
                            path.push("untitled");
                        }
                        vec![format!("file://{}", path.to_string_lossy())]
                    }
                }
                ChooserOptions::SaveFiles(opt) => {
                    if !self.selected_paths.is_empty() {
                        self.selected_paths
                            .iter()
                            .map(|p| format!("file://{}", p.to_string_lossy()))
                            .collect()
                    } else {
                        let mut uris = Vec::new();
                        if let Some(files) = &opt.files {
                            for file_bytes in files {
                                if let Some(file_path) = path_from_bytes(file_bytes) {
                                    uris.push(format!("file://{}", file_path.to_string_lossy()));
                                }
                            }
                        }
                        if uris.is_empty() {
                            let path = self.current_dir.join("untitled");
                            uris.push(format!("file://{}", path.to_string_lossy()));
                        }
                        uris
                    }
                }
            };

            if !selected_uris.is_empty() {
                println!("[ui] Choose clicked: {:?}", selected_uris);
                PENDING_DIALOG.set(
                    Some(FileChooserResponse {
                        handle: self.handle.clone(),
                        outcome: FileChooserOutcome::Selected(selected_uris),
                    })
                );
                return true;
            }
        }

        // 2. Check Cancel button
        if self.cancel_rect != utils::Rect::ZERO && interactivity.is_clicked(self.cancel_rect) {
            println!("[ui] Cancel clicked.");
            PENDING_DIALOG.set(
                Some(FileChooserResponse {
                    handle: self.handle.clone(),
                    outcome: FileChooserOutcome::Cancelled,
                })
            );
            return true;
        }

        // 3. Check Row clicks
        for (id, rect, path) in &self.row_rects {
            if interactivity.is_clicked(*rect) {
                let rows = &self.root.children.0.children.1.children.0;
                if let Some(row_idx) = rows.iter().position(|r| u64::from(r.node_id()) == *id) {
                    let is_dir = rows[row_idx].is_dir;

                    if is_dir {
                        println!("[ui] Navigating to directory {:?}", path);
                        self.current_dir = path.clone();
                        self.selected_paths.clear();
                        self.load_directory(tree);
                    } else {
                        if path.as_os_str().is_empty() {
                            return false;
                        }
                        let is_multiple = match &self.options {
                            ChooserOptions::OpenFile(opt) => opt.multiple.unwrap_or(false),
                            _ => false,
                        };
                        if is_multiple {
                            if let Some(pos) = self.selected_paths.iter().position(|p| p == path) {
                                self.selected_paths.remove(pos);
                            } else {
                                self.selected_paths.push(path.clone());
                            }
                        } else {
                            self.selected_paths = vec![path.clone()];
                        }
                        println!("[ui] Selected paths: {:?}", self.selected_paths);
                        self.load_directory(tree);
                    }
                    return true;
                }
            }
        }

        false
    }

    fn wants_input(&self) -> bool {
        true
    }
}

// --- Layout ------------------------------------------------------------------
fn make_root(
    font_24: &'static BakedFont,
    font_16: &'static BakedFont,
    _handle: &str,
    options: &ChooserOptions
) -> RootDiv {
    let title_str = match options {
        ChooserOptions::OpenFile(opt) => {
            if opt.directory.unwrap_or(false) {
                if opt.multiple.unwrap_or(false) { "Select Folders" } else { "Select Folder" }
            } else if opt.multiple.unwrap_or(false) {
                "Select Files"
            } else {
                "Select File"
            }
        }
        ChooserOptions::SaveFile(_) => "Save File",
        ChooserOptions::SaveFiles(_) => "Save Files",
    };

    let accept_label = match options {
        ChooserOptions::OpenFile(opt) => opt.accept_label.as_deref(),
        ChooserOptions::SaveFile(opt) => opt.accept_label.as_deref(),
        ChooserOptions::SaveFiles(opt) => opt.accept_label.as_deref(),
    };

    let default_label = match options {
        ChooserOptions::OpenFile(opt) => {
            if opt.directory.unwrap_or(false) {
                if opt.multiple.unwrap_or(false) { "Choose Folders" } else { "Choose Folder" }
            } else if opt.multiple.unwrap_or(false) {
                "Choose Files"
            } else {
                "Choose File"
            }
        }
        ChooserOptions::SaveFile(_) => "Save",
        ChooserOptions::SaveFiles(_) => "Save All",
    };

    let mut title = Text::new(Style {
        margin: taffy::Rect {
            left: length(0.0_f32),
            right: length(0.0_f32),
            top: length(0.0_f32),
            bottom: length(6.0_f32),
        },
        ..Default::default()
    });
    title.font = Some(font_24);
    title.text = title_str.to_string();
    title.color = Color::WHITE;
    title.z = 0.95;

    let mut path_text = Text::new(Style::default());
    path_text.font = Some(font_16);
    path_text.text = "Path: ...".to_string();
    path_text.color = Color::rgb(0.6, 0.6, 0.7);
    path_text.z = 0.95;

    let header_style = Style {
        display: Display::Flex,
        flex_direction: FlexDirection::Column,
        size: Size {
            width: percent(1.0_f32),
            height: length(60.0_f32),
        },
        justify_content: Some(JustifyContent::Center),
        align_items: Some(AlignItems::Start),
        ..Default::default()
    };
    let header = Div::new(header_style, (title, path_text));

    let list_style = Style {
        display: Display::Flex,
        flex_direction: FlexDirection::Column,
        size: Size {
            width: percent(1.0_f32),
            height: length(440.0_f32),
        },
        padding: taffy::Rect {
            left: length(8.0_f32),
            right: length(8.0_f32),
            top: length(8.0_f32),
            bottom: length(8.0_f32),
        },
        ..Default::default()
    };
    let mut rows = Vec::new();
    for _ in 0..10 {
        rows.push(FileRow::new(font_16));
    }
    let mut list_div = Div::new(list_style, FileRows(rows));
    list_div.color = Color::rgb(0.02, 0.02, 0.03);
    list_div.border_radius = 8.0;
    list_div.border_color = Color::rgb(0.08, 0.08, 0.1);
    list_div.border_thickness = 1.0;
    list_div.z = 0.5;

    let footer = Footer::new(font_16, accept_label, default_label);

    let modal_style = Style {
        display: Display::Flex,
        flex_direction: FlexDirection::Column,
        justify_content: Some(JustifyContent::SpaceBetween),
        align_items: Some(AlignItems::Center),
        size: Size {
            width: percent(1.0_f32),
            height: percent(1.0_f32),
        },
        padding: taffy::Rect {
            left: length(16.0_f32),
            right: length(16.0_f32),
            top: length(16.0_f32),
            bottom: length(16.0_f32),
        },
        ..Default::default()
    };
    let mut modal = Div::new(modal_style, (header, list_div, footer));
    modal.color = Color::rgb(0.04, 0.04, 0.04);
    modal.border_color = Color::rgb(0.12, 0.12, 0.14);
    modal.border_radius = 0.0;
    modal.border_thickness = 0.0;
    modal.z = 0.2;

    let root_style = Style {
        display: Display::Flex,
        flex_direction: FlexDirection::Column,
        justify_content: Some(JustifyContent::Center),
        align_items: Some(AlignItems::Center),
        size: Size {
            width: percent(1.0_f32),
            height: percent(1.0_f32),
        },
        ..Default::default()
    };
    let mut root = Div::new(root_style, (modal,));
    root.color = Color::rgba(0.0, 0.0, 0.0, 0.6);
    root.z = 0.1;

    root
}
