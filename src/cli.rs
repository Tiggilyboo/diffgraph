use std::{process::Command, io::Write};
use clap::{Arg, ArgAction};
use url::Url;
use std::path::Path;
use regex::Regex;
use unidiff::PatchSet;

use crate::graph::DiffGraphParams;

enum ArgValue {
    Path {
        path: String,
        is_dir: bool,
        exists: bool,
    },
    Url(Url),
    Commit {
        from: String,
        to: String,
    },
}

impl ArgValue {
    pub fn try_parse_url(arg: &str) -> Option<ArgValue> {
        if let Ok(url) = Url::parse(arg) {
            Some(ArgValue::Url(url))
        } else {
            None
        }
    }
    pub fn try_parse_file(arg: &str) -> Option<ArgValue> {
        let path = Path::new(arg);

        Some(ArgValue::Path {
            path: arg.into(),
            is_dir: path.is_dir(),
            exists: path.exists(),
        })
    }
    pub fn try_parse_commit(arg: &str) -> Option<ArgValue> {
        let re = Regex::new(r"^([0-9a-fA-F]+)\.\.([0-9a-fA-F]+)$").unwrap();
        if let Some(captures) = re.captures(arg) {
            Some(ArgValue::Commit { 
                from: captures.get(1).unwrap().as_str().into(),
                to: captures.get(2).unwrap().as_str().into(),
            })
        } else {
            let re = Regex::new(r"^[0-9a-fA-F]{6,64}$").unwrap();
            if re.is_match(arg) {
                Some(ArgValue::Commit { from: "HEAD".into(), to: arg.into() })
            } else {
                None
            }
        }
    }
}

fn try_get_diff_patch(rev_from: &str, rev_to: &str) -> Result<String, String> {
    let cmd_gitdiff = Command::new("git")
        .arg("diff")
        .arg(format!("{}..{}", rev_from, rev_to))
        .output();

    match cmd_gitdiff {
        Ok(output) => Ok(String::from_utf8_lossy(&output.stdout).to_string()),
        Err(e) => Err(e.to_string())
    }
}

fn try_create_patch_set(diff: &str) -> Result<PatchSet, String> {
    let mut patch = PatchSet::new();
    match patch.parse(diff) {
        Ok(_) => Ok(patch),
        Err(e) => Err(e.to_string()),
    }
}

fn try_load_diff_file(file_path: &str) -> Result<String, String> {
    let path = Path::new(file_path);
    match std::fs::read_to_string(path) {
        Ok(file_contents) => Ok(file_contents),
        Err(err) => Err(err.to_string()),
    }
}

fn try_parse_diff(diff_arg: &str) -> Result<Option<PatchSet>, String> {
    let mut diff = match ArgValue::try_parse_commit(&diff_arg) {
        Some(ArgValue::Commit { from, to }) => match try_get_diff_patch(&from, &to) {
            Ok(patch) => Ok(Some(patch)),
            Err(err) => Err(format!("{}", err.to_string())),
        },
        _ => Ok(None),
    };
    let mut has_diff = false;
    match diff {
        Ok(Some(_)) => has_diff = true,
        Ok(None) => has_diff = false,
        Err(e) => return Err(e.to_string()),
    }
    if !has_diff {
        diff = match ArgValue::try_parse_file(&diff_arg) {
            Some(ArgValue::Path { path, is_dir, exists }) => {
                if exists {
                    if is_dir {
                        Err(format!("diff path must be a file, directory is not supported at the moment..."))
                    } else {
                        match try_load_diff_file(&path) {
                            Ok(diff) => Ok(Some(diff)),
                            Err(err) => Err(err.to_string()) 
                        }
                    }
                } else {
                    Err(format!("diff path '{}' does not exist.", path))
                }
            },
            _ => Ok(None),
        };
    }

    if let Ok(Some(diff)) = diff {
        match try_create_patch_set(&diff) {
            Ok(patch) => Ok(Some(patch)),
            Err(e) => Err(e),
        }
    } else {
        Ok(None)
    }
}

fn dir_is_git_repository(dir: &str) -> bool {
    let output = Command::new("git")
        .arg("rev-parse")
        .arg("--is-inside-work-tree")
        .current_dir(dir)
        .output()
        .expect("Failed to execute git command");

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    stdout.trim() == "true" && stderr.is_empty()
}

fn try_clone_repo(url: &str, clone_path: &str) -> Result<String, String> {
    dbg!(url, clone_path);

    let output = Command::new("git")
        .arg("clone")
        .arg(url)
        .arg(clone_path)
        .output()
        .expect("Failed to execute git clone command");

    if output.status.success() {
        Ok(clone_path.into())
    } else {
        let error_message = String::from_utf8_lossy(&output.stderr).to_string();
        Err(error_message)
    }
}

fn try_parse_repo(repo_arg: &str, clone_path: Option<String>) -> Result<Option<String>, String> {
    fn fallback_value(url: &Url) -> String {
        if url.path().len() > 0 {
            url.path().to_string()
        } else {
            ".".into()
        }
    }

    match ArgValue::try_parse_url(&repo_arg) {
        Some(ArgValue::Url(url)) => {
            let clone_path = if let Some(clone_path) = clone_path {
                clone_path
            } else {
                // Use the git name at the end of the URL
                match url.path_segments() {
                    Some(url_segments) => {
                        if let Some(last_segment) = url_segments.last() {
                            // Trim ".git" if there is one
                            if last_segment.ends_with(".git") {
                                last_segment[..last_segment.len()-4].to_string()
                            } else {
                                last_segment.into()
                            }
                        } else {
                            fallback_value(&url)
                        }
                    },
                    None => fallback_value(&url)
                }
            };
            return match try_clone_repo(&url.as_str(), &clone_path) {
                Ok(repo_path) => Ok(Some(repo_path)),
                Err(e) => Err(e),
            }
        },
        _ => (),
    }

    match ArgValue::try_parse_file(&repo_arg) {
        Some(ArgValue::Path { path, is_dir, exists }) => {
            if exists {
                if is_dir {
                    if dir_is_git_repository(&path) {
                        Ok(Some(path))
                    } else {
                        Err(format!("Repository path '{}' is not a git repository", path))
                    }
                } else {
                    Err(format!("Repository path '{}' must be a directory", path))
                }
            } else {
                Err(format!("Repository path '{}' does not exist", path))
            }
        },
        _ => Ok(None),
    }
}

pub fn get_params() -> Result<DiffGraphParams, String> {
    let matches = clap::Command::new("diffdiagram")
        .arg(Arg::new("repo")
            .short('r')
            .long("repository")
            .value_name("URL or PATH")
            .default_value(".")
            .required(true)
            .help("Specify a URL or path to repository to diff against"))
        .arg(Arg::new("clone")
            .requires("repo")
            .short('c')
            .long("clone-path")
            .value_name("PATH")
            .help("Specify a clone path for the diff repository to clone to"))
        .arg(Arg::new("diff")
            .short('d')
            .long("diff")
            .value_name("PATCH FILE or GIT REVISIONS")
            .required(true)
            .help("Specify diff patch file or git revision to create a diff"))
        .arg(Arg::new("install-missing")
            .short('i')
            .long("install-missing")
            .action(ArgAction::SetTrue)
            .help("Install missing tree-sitter parsers automatically"))
            
        .get_matches();

    let clone_path = matches.get_one::<String>("clone");
    let repo_arg = matches.get_one::<String>("repo").unwrap();
    let repo = match try_parse_repo(repo_arg, clone_path.cloned()) {
        Ok(repo) => {
            println!("Repository path: {:?}", repo);
            repo
        },
        Err(err) => {
            println!("{}", err);
            None
        },
    };
    
    let diff_arg = matches.get_one::<String>("diff").unwrap();
    let diff = match try_parse_diff(diff_arg) {
        Ok(diff) => diff,
        Err(err) => {
            println!("{}", err);
            None
        }
    };

    let install_lang_if_missing = matches.get_flag("install-missing");

    if let Some(diff) = diff {
        if let Some(repo) = repo {
            Ok(DiffGraphParams { 
                diff_repository_dir: repo, 
                diff, 
                install_lang_if_missing,
                save_default_if_missing: true,
            })
        } else {
            Err("No repo given".into())
        }
    } else {
        Err("No diff given".into())
    }
}
