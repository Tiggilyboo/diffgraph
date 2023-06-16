use std::process::Command;
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
    if let Some(arg) = matches.get_one::<String>("diff") {
        match ArgValue::try_parse_commit(&arg) {
            Some(ArgValue::Commit { .. }) => match try_get_diff_patch(arg) {
                Ok(patch) => {
                    println!("Diffing: {}", patch);
                },
                Err(err) => panic!("{}", err),
            },
            _ => (),
        }
        match ArgValue::try_parse_file(&arg) {
            Some(ArgValue::Path { path, is_dir, exists }) => {

            },
            _ => help(),
        }
    }


}
