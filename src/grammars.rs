use tree_sitter::{Language, Parser, Tree, InputEdit};
use tree_sitter_loader::*;
use std::{path::Path, borrow::BorrowMut};

pub struct Grammars {
    loader: Loader,
}


impl Grammars {
    pub fn load(dirs: Vec<&str>) -> Result<Self, String> {
        let mut loader = Loader::new().map_err(|e| e.to_string())?;
        let config = if dirs.len() > 0 {
            let mut c = Config::default();
            for d in dirs {
                c.parser_directories.push(d.into());
            }
            c
        } else {
            Config::initial()
        };

        loader.find_all_languages(&config).map_err(|e| e.to_string())?;

        Ok(Self {
            loader,
        })
    }

    pub fn try_get_language(&self, path: &Path) -> Result<Option<Language>, String> {
        match self.loader.language_configuration_for_file_name(path).map_err(|e| e.to_string())? {
            Some((lang, _)) => Ok(Some(lang)),
            None => Ok(None),
        }
    }

}
