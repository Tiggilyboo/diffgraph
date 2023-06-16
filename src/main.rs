use std::{process::Command, io::Write};
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

fn help() {
    println!("Usage: ");
    println!("diffdiagram --repository <VALUE> --diff <VALUE>");
    println!("--repository \t\tURL (ie. http://github.com/user/repo.git");
    println!("--repository \t\tLocal Path (ie '.' for local directory)");
    println!();
    println!("--diff\t\t Current revision (HEAD) to git hash: (ex. 2ef7bd)");
    println!("--diff\t\t Git diff 2ef7bd..de3f11)");
    println!("--diff\t\t Patch file (ie. ./path/to/patch/file.diff)");
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

fn main() {
    let matches = clap::Command::new("diffdiagram")
        .arg(clap::arg!(--repository <VALUE>).required(true))
        .arg(clap::arg!(--diff <VALUE>).required(true))
        .get_matches();

    if let Some(arg) = matches.get_one::<String>("repository") {
        match ArgValue::try_parse_url(&arg) {
            Some(ArgValue::Url(url)) => {

            },
            _ => (),
        }
        match ArgValue::try_parse_file(&arg) {
            Some(ArgValue::Path { path, is_dir, exists }) => {

            },
            _ => help(),
        }
    }
    
    let diff_arg = matches.get_one::<String>("diff").unwrap();
    let diff = try_parse_diff(diff_arg);
    match diff {
        Ok(diff) => println!("{:?}", diff),
        Err(err) => println!("{}", err),
    };
}
