use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use unidiff::{PatchSet, PatchedFile, LINE_TYPE_ADDED, LINE_TYPE_REMOVED, LINE_TYPE_CONTEXT };
use tree_sitter::{Parser, Tree, Point, InputEdit, Language};

use crate::grammars::Grammars;

#[derive(Debug)]
struct LineByteCounter<'a> {
    byte_count: usize,
    lines: std::str::Lines<'a>,
    cache: Vec<(usize, &'a str)>,
}

impl<'a> LineByteCounter<'a> {
    fn new(content: &'a str) -> LineByteCounter<'a> {
        let lines = content.lines();

        LineByteCounter { 
            lines, 
            byte_count: 0,
            cache: Vec::new(),
        }
    }
    
    fn get(&self, line_num: usize) -> Option<&(usize, &'a str)> {
        self.cache.get(line_num)
    }

    fn last_in_cache(&self) -> Option<(usize, usize, &'a str)> {
        if let Some(last) = self.cache.last() {
            Some((self.cache.len() - 1, last.0, last.1))
        } else {
            None
        }
    }
}

impl<'a> Iterator for LineByteCounter<'a> {
    type Item = (usize, &'a str);

    fn next(&mut self) -> Option<Self::Item> {
        let line = self.lines.next()?;
        let byte_count = self.byte_count;

        self.byte_count += line.len() + '\n'.len_utf8();

        let item = (byte_count, line);
        self.cache.push(item);

        Some(item)
    }
}

#[derive(Debug)]
pub struct Diff {
    pub source: String,
    pub source_file: String,
    pub target_file: String,
    pub source_file_path: String,
    pub edits: Vec<InputEdit>,
    pub tree: Tree,
    pub language: Language,
}

fn get_fs_file_path<'a>(patch_file_path: &'a str) -> &'a str {
    let file = if let Some(stripped_path) = patch_file_path.strip_prefix("a/") {
        stripped_path
    } else if let Some(stripped_path) = patch_file_path.strip_prefix("b/") {
        stripped_path
    } else {
        patch_file_path
    };

    file
}

fn try_load_file_from(file_path: &str) -> Result<String, String> {
    let path = Path::new(file_path);
    if !path.exists() {
        return Err(format!("'{}' does not exist", file_path));
    }
    if !path.is_file() {
        return Err(format!("'{}' is not a file", file_path))
    }
    match std::fs::read_to_string(path) {
        Ok(contents) => Ok(contents),
        Err(e) => Err(e.to_string())
    }
}

impl Diff {
    pub fn from_patch_file(patch_file: &PatchedFile, grammars: &Grammars) -> Result<Self, String> {

        // Load the source file from disk to get byte counts
        // And later use to parse the entire tree
        let source: String;

        // Trim off the a/ or b/ from the file
        let source_file_path = get_fs_file_path(&patch_file.source_file);
        match try_load_file_from(source_file_path) {
            Ok(contents) => source = contents,
            Err(e) => return Err(e),
        }

        let mut edits = Vec::new();
        let mut line_byte_counter = LineByteCounter::new(&source);

        // TODO: Do some funky character specific diff combination instead of just line diffs? 

        for hunk in patch_file.hunks() {

            let mut last_source_context_line = hunk.source_start;
            for line in hunk.lines() {
                println!("{:?}", line);
                
                match line.line_type.as_str() {
                    LINE_TYPE_ADDED | LINE_TYPE_REMOVED => {
                        if let Some(source_line_no) = line.source_line_no {
                            let (start_byte, source_line_str) = if let Some(cached_line) = line_byte_counter.get(source_line_no) {
                                *cached_line
                            } else {
                                let mut item = None;
                                let iterations_to_go = if let Some(last) = line_byte_counter.last_in_cache() {
                                    item = Some((last.1, last.2));
                                    last_source_context_line + 1 - last.0
                                } else {
                                    last_source_context_line + 1
                                };
                                for _ in 0..iterations_to_go {
                                    if let Some(counter) = line_byte_counter.next() {
                                        item = Some(counter);
                                    } else {
                                        return Err(format!("Line counter could not iterate {}, ran out of lines", iterations_to_go))
                                    }
                                }
                                if let Some(item) = item {
                                    item
                                } else {
                                    return Err(format!("Unable to determine line start byte count for source line {} (L{} in diff)", source_line_no, line.diff_line_no))
                                }
                            };
                            let old_end_byte = start_byte + source_line_str.len();
                            let new_end_byte = start_byte + line.value.len();
                            let new_end_row = if let Some(new_end_row) = line.target_line_no {
                                new_end_row
                            } else {
                                // Line was removed, 
                                assert_eq!(source_line_no - 1, last_source_context_line);
                                source_line_no - 1
                            };

                            edits.push(InputEdit { 
                                start_byte, 
                                old_end_byte, 
                                new_end_byte, 
                                start_position: Point { row: source_line_no, column: 0 }, 
                                old_end_position: Point { row: source_line_no, column: source_line_str.chars().count() }, 
                                new_end_position: Point { row: new_end_row, column: line.value.chars().count() } 
                            });
                            last_source_context_line = source_line_no;
                        } else {
                        }
                    },
                    LINE_TYPE_CONTEXT => {
                        if let Some(source_line_no) = line.source_line_no {
                            last_source_context_line = source_line_no;
                        } else {
                            return Err(format!("Context line {} in patch requires source line", line.diff_line_no));
                        }
                    },
                    _ => continue,
                }
                
            }
        }
    
        let source_file = patch_file.source_file.clone();
        let target_file = patch_file.target_file.clone();
        let source_file_path = source_file_path.to_string();

        let tree_path = Path::new(&source_file_path);
        dbg!(tree_path);
        let lang = grammars.try_get_language(tree_path).map_err(|e| e.to_string())?;
        dbg!(lang);

        let tree: Tree;
        if let Some(lang) = lang {
            tree = match try_parse_source_code(lang, &source) {
                Ok(Some(tree)) => tree,
                Ok(None) => return Err(format!("Unable to parse patch file: {}", patch_file.path())),
                Err(e) => return Err(e),
            };
        } else {
            return Err(format!("Unable to determine language using tree-sitter parsers for file {}.\nCurrently configured tree-sitter paths: {:?}", 
                tree_path.display(), grammars.get_configured_paths()));
        }
        let language = tree.language();

        Ok(Self {
            source,
            source_file,
            source_file_path,
            target_file,
            edits,
            tree,
            language,
        })
    }

    fn try_apply_edits(&mut self) -> Result<Tree, String> {
        let mut tree = self.tree.clone();
        for edit in self.edits.iter() {
            tree.edit(edit);
        }
        Ok(tree)
    }
}

fn export_tree_to_dot(tree: &Option<Tree>) -> Result<(), String> {
    if let Some(tree) = tree {
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        let path_str = format!("./tree{}.dot", now.as_secs());
        let file = std::fs::File::create(Path::new(&path_str)).map_err(|e| e.to_string())?;
        tree.print_dot_graph(&file);
        Ok(())
    } else {
        Err("No tree has been parsed yet".into())
    }
}

pub fn try_parse_source_code(language: Language, source_code: &str) -> Result<Option<Tree>, String> {
    let mut parser = Parser::new();
    parser.set_language(language).map_err(|e| e.to_string())?;

    let timeout_micros = 1_000_000;
    parser.set_timeout_micros(timeout_micros);

    let tree = parser.parse(source_code, None);

    Ok(tree)
}

pub fn try_parse_patch(
    patch: &PatchSet, 
    parser_config_path: Option<PathBuf>, 
    save_default_if_missing: bool, 
    install_lang_if_missing: bool
) -> Result<Vec<Diff>, String> {

    let grammars = Grammars::load(parser_config_path, save_default_if_missing).map_err(|e| e.to_string())?;
    if install_lang_if_missing {
        println!("Checking missing languages...");
        grammars.try_install_languages()?;
    }

    let mut diffs = Vec::new();
    for patch_file in patch.files() {
        match Diff::from_patch_file(patch_file, &grammars) {
            Ok(mut diff) => {
                let _diff_tree = diff.try_apply_edits()?;
                diffs.push(diff);
            },
            Err(e) => return Err(e),
        }
    }

    Ok(diffs)
}
