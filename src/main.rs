use std::{process::Command, io::Write};
use clap::Arg;
use url::Url;
use std::path::Path;
use regex::Regex;

enum ArgValue {
    Path {
        path: String,
        is_dir: bool,
        exists: bool,
    },
    Url(String),
    Commit {
        from: String,
        to: String,
    },
}

impl ArgValue {
    pub fn try_parse_url(arg: &str) -> Option<ArgValue> {
        if let Ok(url) = Url::parse(arg) {
            Some(ArgValue::Url(arg.into()))
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

fn try_get_diff_patch(diff_args: &str) -> Result<String, String> {
    let cmd_gitdiff = Command::new("git")
        .arg("diff")
        .arg(diff_args)
        .output();

    match cmd_gitdiff {
        Ok(output) => Ok(String::from_utf8_lossy(&output.stdout).to_string()),
        Err(e) => Err(e.to_string())
    }
}

fn try_validate_diff_patch(patch: &str) -> Result<(), String> {
    let mut cmd_gitapply = Command::new("git")
        .args(&["apply", "--numstat", "--summary", "--check", "-"])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("Failed to validate git diff, are you sure the diff can apply to this repository?");

    if let Some(stdin) = cmd_gitapply.stdin.as_mut() {
        stdin.write_all(patch.as_bytes())
            .expect("Failed to write patch to stdin");
    }

    let output = cmd_gitapply.wait_with_output()
        .expect("Failed to read git apply command");

    let err_str = String::from_utf8_lossy(&output.stderr).to_string();
    if err_str.len() > 0 {
        return Err(err_str);
    }

    let out_str = String::from_utf8_lossy(&output.stdout).to_string();
    if out_str.len() > 0 {
        println!("Validated patch: {}", out_str);
        Ok(())
    }
    else {
        Err("error: Unexpected response from git apply to validate diff".into())
    }
}

fn try_load_diff_file(file_path: &str) -> Result<String, String> {
    let path = Path::new(file_path);
    match std::fs::read_to_string(path) {
        Ok(file_contents) => Ok(file_contents),
        Err(err) => Err(err.to_string()),
    }
}

fn try_parse_diff(diff_arg: &str) -> Result<Option<String>, String> {
    let diff = match ArgValue::try_parse_commit(&diff_arg) {
        Some(ArgValue::Commit { .. }) => match try_get_diff_patch(diff_arg) {
            Ok(patch) => Ok(Some(patch)),
            Err(err) => Err(format!("{}", err.to_string())),
        },
        _ => Ok(None),
    };
    if let Ok(Some(diff)) = diff {
        return Ok(Some(diff))
    }
    let diff = match ArgValue::try_parse_file(&diff_arg) {
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

    if let Ok(Some(diff)) = diff {
        match try_validate_diff_patch(&diff) {
            Ok(()) => Ok(Some(diff)),
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
    match ArgValue::try_parse_url(&repo_arg) {
        Some(ArgValue::Url(url)) => {
            let clone_path = if let Some(clone_path) = clone_path {
                clone_path
            } else {
                ".".into()
            };
            return match try_clone_repo(&url, &clone_path) {
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

fn main() {
    let matches = clap::Command::new("diffdiagram")
        .arg(Arg::new("repo")
            .short('r')
            .long("repository")
            .value_name("URL or PATH")
            .required(true))
        .arg(Arg::new("diff")
            .short('d')
            .long("diff")
            .value_name("PATCH FILE or GIT REVISIONS")
            .required(true))
        .arg(Arg::new("clone")
            .requires("repo")
            .short('c')
            .long("clone-path")
            .value_name("PATH"))
        .get_matches();

    let clone_path = matches.get_one::<String>("clone");
    let repo_arg = matches.get_one::<String>("repo").unwrap();
    let repo = try_parse_repo(repo_arg, clone_path.cloned());
    match repo {
        Ok(Some(repo)) => println!("repo: {}", repo),
        Ok(None) => println!(""),
        Err(err) => println!("{}", err),
    }
    
    let diff_arg = matches.get_one::<String>("diff").unwrap();
    let diff = try_parse_diff(diff_arg);
    match diff {
        Ok(diff) => println!("{:?}", diff),
        Err(err) => println!("{}", err),
    };
}
