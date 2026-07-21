use crate::backend::{ OpenFileOptions, SaveFileOptions, SaveFilesOptions };

#[derive(Debug, Clone)]
pub enum ChooserOptions {
    OpenFile(OpenFileOptions),
    SaveFile(SaveFileOptions),
    SaveFiles(SaveFilesOptions),
}
