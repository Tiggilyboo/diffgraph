use std::path::Path;
use unidiff::{PatchSet, PatchedFile, LINE_TYPE_ADDED, LINE_TYPE_REMOVED };
use tree_sitter::{Parser, Tree, Point, InputEdit, Language};

use crate::grammars::Grammars;

#[derive(Debug)]
struct LineByteCounter<'a> {
    lines: std::str::Lines<'a>,
    byte_count: usize,
}

impl<'a> LineByteCounter<'a> {
    fn new(content: &'a str) -> LineByteCounter<'a> {
        LineByteCounter { 
            lines: content.lines(), 
            byte_count: 0 
        }
    }
}
impl<'a> Iterator for LineByteCounter<'a> {
    type Item = (usize, &'a str);

    fn next(&mut self) -> Option<Self::Item> {
        let line = self.lines.next()?;
        let byte_count = self.byte_count;

        self.byte_count += line.len() + '\n'.len_utf8();

        Some((byte_count, line))
    }
}

#[derive(Debug)]
pub struct Diff {
    pub source: String,
    pub source_file: String,
    pub target_file: String,
    pub source_file_path: String,
    pub added: Vec<InputEdit>,
    pub removed: Vec<InputEdit>,
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

fn get_file_language(file_path: &str) -> Option<Language> {
    None
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

    pub fn from_patch_file(patch_file: &PatchedFile) -> Result<Self, String> {

        // Load the source file from disk to get byte counts
        // And later use to parse the entire tree
        let source: String;

        // Trim off the a/ or b/ from the file
        let source_file_path = get_fs_file_path(&patch_file.source_file);
        match try_load_file_from(source_file_path) {
            Ok(contents) => source = contents,
            Err(e) => return Err(e),
        }

        let mut line_byte_counter = LineByteCounter::new(&source).peekable();
        let mut added = Vec::new();
        let mut removed = Vec::new();

        // TODO: Do some funky character specific diff combination instead of just line diffs? 

        for hunk in patch_file.hunks() {

            let mut last_line_num = 0;
            for line in hunk.lines() {

                println!("{:?}", line);

                let old_line_num = line.source_line_no;
                let new_line_num = line.target_line_no;

                let line_diff = if let Some(old_line_num) = old_line_num {
                    let next_diff = old_line_num - last_line_num;
                    last_line_num = old_line_num;

                    next_diff
                } else {
                    0
                };

                let current_line_counter = {
                    let mut at_next_diff = line_byte_counter.peek().cloned();
                    for _ in 1..line_diff {
                        at_next_diff = line_byte_counter.next()
                    };
                    at_next_diff
                };

                if current_line_counter.is_none() {
                    return Err(format!("Unable to find line byte offset for line number: {:?}", old_line_num))
                }
                let (start_byte, old_line) = current_line_counter.unwrap();
                
                // No reason to continue after this, skip
                match line.line_type.as_str() {
                    LINE_TYPE_ADDED | LINE_TYPE_REMOVED => (),
                    _ => continue,
                };

                let start_line_num = old_line_num.or_else(|| new_line_num);
                if start_line_num.is_none() {
                    dbg!(line);
                    return Err(format!("Start line number could not be determined in diff line: {}", line.diff_line_no))
                }
                let start_line_num = start_line_num.unwrap();
                let old_line_len_bytes = old_line.len();
                let old_line_len = old_line.chars().count();
                let new_line_len_bytes = line.value.len();
                let new_line_len = line.value.chars().count();

                // Old line does not exist in this line, we are adding one
                //  Also add the last line length before this one
                let last_line_len_bytes = if old_line_num.is_none() {
                    old_line_len_bytes
                } else {
                    0
                };
                let edit = InputEdit {
                    start_byte,
                    start_position: Point { row: start_line_num, column: 0 },
                    old_end_byte: start_byte + old_line_len_bytes,
                    old_end_position: Point { row: old_line_num.unwrap_or(last_line_num), column: old_line_len },
                    new_end_byte: start_byte + last_line_len_bytes + new_line_len_bytes,
                    new_end_position: Point { row: new_line_num.unwrap_or(start_line_num), column: new_line_len },
                };

                match line.line_type.as_str() {
                    LINE_TYPE_ADDED => added.push(edit),
                    LINE_TYPE_REMOVED => removed.push(edit),
                    _ => continue,
                };
            }
        }
    
        let source_file = patch_file.source_file.clone();
        let target_file = patch_file.target_file.clone();
        let source_file_path = source_file_path.to_string();

        Ok(Self {
            source,
            source_file,
            source_file_path,
            target_file,
            added,
            removed
        })
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

pub fn try_parse_patch(patch: &PatchSet) -> Result<Vec<Tree>, String> {
    let grammars = Grammars::load(Vec::new()).map_err(|e| e.to_string())?;
    let mut trees = Vec::new();

    for patch_file in patch.files() {
        match Diff::from_patch_file(patch_file) {
            Ok(diff) => {
                let source_file_path = Path::new(&diff.source_file_path);
                if let Some(lang) = grammars.try_get_language(source_file_path).map_err(|e| e.to_string())? {
                    dbg!(lang);
                    match try_parse_source_code(lang, &diff.source) {
                        Ok(Some(tree)) => trees.push(tree),
                        Ok(None) => return Err(format!("Unable to parse patch file: {}", patch_file.path())),
                        Err(e) => return Err(e),
                    }
                }
            },
            Err(e) => return Err(e),
        }
    }

    Ok(trees)
}
