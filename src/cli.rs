use std::process::Command;
use clap::{Arg, ArgAction};
use url::Url;
use std::path::{Path, PathBuf};
use regex::Regex;
use unidiff::PatchSet;

use crate::graph::DiffGraphParams;

#[derive(Debug)]
enum ArgValue {
    Path {
        path: PathBuf,
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

fn try_check_apply_patch(file_path: &PathBuf, repo_path: &PathBuf) -> Result<bool, String> {
    let cmd_gitapply = Command::new("git")
        .arg("apply")
        .arg("--check")
        .arg(file_path)
        .current_dir(repo_path)
        .status()
        .map_err(|e| e.to_string())?;

    Ok(cmd_gitapply.success())
}

fn try_create_patch_set(diff: &str) -> Result<PatchSet, String> {
    let mut patch = PatchSet::new();
    match patch.parse(diff) {
        Ok(_) => Ok(patch),
        Err(e) => Err(e.to_string()),
    }
}

fn try_load_diff_file(file_path: &PathBuf) -> Result<String, String> {
    let path = Path::new(file_path);
    match std::fs::read_to_string(path) {
        Ok(file_contents) => Ok(file_contents),
        Err(err) => Err(err.to_string()),
    }
}

fn try_parse_diff(diff_arg: &str, repo_path: &PathBuf) -> Result<PatchSet, String> {
    let diff_from_commit;
    match ArgValue::try_parse_commit(&diff_arg) {
        Some(ArgValue::Commit { from, to }) => match try_get_diff_patch(&from, &to) {
            Ok(patch) => diff_from_commit = Some(patch),
            Err(err) => return Err(err.to_string()),
        },
        None => diff_from_commit = None,
        Some(unsupported_arg) => return Err(format!("Unsupported type [{:?}] from argument {}", unsupported_arg, diff_arg)),
    };
    let diff;
    if let Some(diff_from_commit) = diff_from_commit {
        diff = diff_from_commit;
    } else {
        diff = match ArgValue::try_parse_file(&diff_arg) {
            Some(ArgValue::Path { path, is_dir, exists }) => {
                if exists {
                    if is_dir {
                        return Err(format!("diff path must be a file, directory is not supported at the moment..."))
                    } else {
                        // Check that the file can apply to our repository
                        if !try_check_apply_patch(&path, repo_path)? {
                            return Err(format!("diff '{:?}' could not be applied to repository at {:?}", path, repo_path.display()));
                        }
                        // Load it
                        match try_load_diff_file(&path) {
                            Ok(diff) => diff,
                            Err(err) => return Err(err.to_string()) 
                        }
                    }
                } else {
                    return Err(format!("diff path '{:?}' does not exist.", path))
                }
            },
            _ => return Err(format!("Unable to parse diff from argument '{}'", diff_arg)),
        };
    }

    match try_create_patch_set(&diff) {
        Ok(patch) => Ok(patch),
        Err(e) => Err(e),
    }
}

fn dir_is_git_repository(dir: &PathBuf) -> bool {
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

fn try_clone_repo(url: &str, clone_path: &str) -> Result<PathBuf, String> {
    dbg!(url, clone_path);

    let output = Command::new("git")
        .arg("clone")
        .arg(url)
        .arg(clone_path)
        .output()
        .expect("Failed to execute git clone command");

    if output.status.success() {
        Ok(Path::new(clone_path).to_path_buf())
    } else {
        let error_message = String::from_utf8_lossy(&output.stderr).to_string();
        Err(error_message)
    }
}

fn try_parse_repo(repo_arg: &str, clone_path: Option<String>) -> Result<Option<PathBuf>, String> {
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
                        Err(format!("Repository path '{:?}' is not a git repository", path))
                    }
                } else {
                    Err(format!("Repository path '{:?}' must be a directory", path))
                }
            } else {
                Err(format!("Repository path '{:?}' does not exist", path))
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
    let repository_path;
    match try_parse_repo(repo_arg, clone_path.cloned()) {
        Ok(Some(repo)) => {
            println!("Repository path: {:?}", repo);
            repository_path = repo;
        },
        Ok(None) => return Err(format!("No repository found at {}", repo_arg)),
        Err(e) => return Err(e.to_string()),
    };
    
    let diff_arg = matches.get_one::<String>("diff").unwrap();
    let diff;
    match try_parse_diff(diff_arg, &repository_path) {
        Ok(parsed_diff) => diff = parsed_diff,
        Err(err) => return Err(err.to_string()),
    };

    let install_lang_if_missing = matches.get_flag("install-missing");

    if let Some(repo_path_str) = repository_path.to_str() { 
        Ok(DiffGraphParams { 
            diff_repository_dir: repo_path_str.to_string(),
            diff, 
            install_lang_if_missing,
            save_default_if_missing: true,
        })
    } else {
        Err(format!("Unable to convert repository path: {}", repository_path.display()))
    }
}
