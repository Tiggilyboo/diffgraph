use tree_sitter::Language;
use tree_sitter_loader::*;
use url::Url;
use std::path::{Path, PathBuf};
use serde::{Serialize, Deserialize};

const TREE_SITTER_CONFIG_FILE: &'static str = "config.json";
const PARSERS_CONFIG_FILE: &'static str = "parsers.json";
const PARSERS_PATH: &'static str = "parsers";

pub struct Grammars {
    loader: Loader,
    ts_config: Config,
    parser_config: ParserConfig,
}

#[derive(Serialize, Deserialize)]
pub struct ParserConfig {
    pub parsers: Vec<String>,
}

fn get_default_config_dir() -> Option<PathBuf> {
    if let Some(path) = dirs::config_dir() {
        Some(path.join("tree-sitter"))
    } else {
        None
    }
}
fn get_default_parsers_dir() -> Option<PathBuf> {
    if let Some(path) = get_default_config_dir() {
        Some(path.join(PARSERS_PATH))
    } else {
        None
    }
}

fn try_get_parser_repo_path(parser_url: &str) -> Result<PathBuf, String> {
    if let Some(path) = get_default_parsers_dir() {
        let url = Url::parse(parser_url).map_err(|e| e.to_string())?; 
        let repo_path; 
        if let Some(segments) = url.path_segments() {
            if let Some(last_segment) = segments.last() {
                repo_path = path.join(last_segment)
            } else {
                return Err(format!("Unable to determine last path segment for repository URL: {}", url));
            }
        } else {
            return Err(format!("Unable to determine path for repository URL: {}", url));
        }

        dbg!(&repo_path);

        Ok(repo_path)
    } else {
        return Err(format!("Unable to determine default parser path."));
    }
}


impl ParserConfig {
    fn create_with_known() -> Result<Self, String> {
        let parsers: Vec::<String> = vec![
            // https://tree-sitter.github.io/tree-sitter/#parsers
            "https://github.com/briot/tree-sitter-ada".into(),
            "https://github.com/tree-sitter/tree-sitter-agda".into(),
            "https://github.com/aheber/tree-sitter-sfapex".into(),
            "https://github.com/tree-sitter/tree-sitter-bash".into(),
            "https://github.com/zwpaper/tree-sitter-beancount".into(),
            "https://github.com/amaanq/tree-sitter-capnp".into(),
            "https://github.com/tree-sitter/tree-sitter-c".into(),
            "https://github.com/tree-sitter/tree-sitter-cpp".into(),
            "https://github.com/tree-sitter/tree-sitter-c-sharp".into(),
            "https://github.com/sogaiu/tree-sitter-clojure".into(),
            "https://github.com/uyha/tree-sitter-cmake".into(),
            "https://github.com/stsewd/tree-sitter-comment".into(),
            "https://github.com/theHamsta/tree-sitter-commonlisp".into(),
            "https://github.com/tree-sitter/tree-sitter-css".into(),
            "https://github.com/theHamsta/tree-sitter-cuda".into(),
            "https://github.com/UserNobody14/tree-sitter-dart".into(),
            "https://github.com/gdamore/tree-sitter-d".into(),
            "https://github.com/camdencheek/tree-sitter-dockerfile".into(),
            "https://github.com/rydesun/tree-sitter-dot".into(),
            "https://github.com/elixir-lang/tree-sitter-elixir".into(),
            "https://github.com/elm-tooling/tree-sitter-elm".into(),
            "https://github.com/Wilfred/tree-sitter-elisp".into(),
            "https://github.com/eno-lang/tree-sitter-eno".into(),
            "https://github.com/tree-sitter/tree-sitter-embedded-template".into(),
            "https://github.com/WhatsApp/tree-sitter-erlang/".into(),
            "https://github.com/travonted/tree-sitter-fennel".into(),
            "https://github.com/ram02z/tree-sitter-fish".into(),
            "https://github.com/siraben/tree-sitter-formula".into(),
            "https://github.com/stadelmanma/tree-sitter-fortran".into(),
            "https://github.com/ObserverOfTime/tree-sitter-gitattributes".into(),
            "https://github.com/shunsambongi/tree-sitter-gitignore".into(),
            "https://github.com/gleam-lang/tree-sitter-gleam".into(),
            "https://github.com/theHamsta/tree-sitter-glsl".into(),
            "https://github.com/tree-sitter/tree-sitter-go".into(),
            "https://github.com/camdencheek/tree-sitter-go-mod".into(),
            "https://github.com/omertuc/tree-sitter-go-work".into(),
            "https://github.com/bkegley/tree-sitter-graphql".into(),
            "https://github.com/slackhq/tree-sitter-hack".into(),
            "https://github.com/tree-sitter/tree-sitter-haskell".into(),
            "https://github.com/MichaHoffmann/tree-sitter-hcl".into(),
            "https://github.com/tree-sitter/tree-sitter-html".into(),
            "https://github.com/tree-sitter/tree-sitter-java".into(),
            "https://github.com/tree-sitter/tree-sitter-javascript".into(),
            "https://github.com/flurie/tree-sitter-jq".into(),
            "https://github.com/Joakker/tree-sitter-json5".into(),
            "https://github.com/tree-sitter/tree-sitter-json".into(),
            "https://github.com/tree-sitter/tree-sitter-julia".into(),
            "https://github.com/fwcd/tree-sitter-kotlin".into(),
            "https://github.com/traxys/tree-sitter-lalrpop".into(),
            "https://github.com/latex-lsp/tree-sitter-latex".into(),
            "https://github.com/Julian/tree-sitter-lean".into(),
            "https://github.com/benwilliamgraham/tree-sitter-llvm".into(),
            "https://github.com/Flakebi/tree-sitter-llvm-mir".into(),
            "https://github.com/Flakebi/tree-sitter-tablegen".into(),
            "https://github.com/Azganoth/tree-sitter-lua".into(),
            "https://github.com/alemuller/tree-sitter-make".into(),
            "https://github.com/ikatyang/tree-sitter-markdown".into(),
            "https://github.com/MDeiml/tree-sitter-markdown".into(),
            "https://github.com/Decodetalkers/tree-sitter-meson".into(),
            "https://github.com/staysail/tree-sitter-meson".into(),
            "https://github.com/grahambates/tree-sitter-m68k".into(),
            "https://github.com/cstrahan/tree-sitter-nix".into(),
            "https://github.com/jiyee/tree-sitter-objc".into(),
            "https://github.com/tree-sitter/tree-sitter-ocaml".into(),
            "https://github.com/milisims/tree-sitter-org".into(),
            "https://github.com/Isopod/tree-sitter-pascal".into(),
            "https://github.com/ganezdragon/tree-sitter-perl".into(),
            "https://github.com/tree-sitter-perl/tree-sitter-perl".into(),
            "https://github.com/tree-sitter-perl/tree-sitter-pod".into(),
            "https://github.com/tree-sitter/tree-sitter-php".into(),
            "https://github.com/rolandwalker/tree-sitter-pgn".into(),
            "https://github.com/PowerShell/tree-sitter-PowerShell".into(),
            "https://github.com/mitchellh/tree-sitter-proto".into(),
            "https://github.com/tree-sitter/tree-sitter-python".into(),
            "https://github.com/yuja/tree-sitter-qmljs".into(),
            "https://github.com/6cdh/tree-sitter-racket".into(),
            "https://github.com/Fymyte/tree-sitter-rasi".into(),
            "https://github.com/alemuller/tree-sitter-re2c".into(),
            "https://github.com/tree-sitter/tree-sitter-regex".into(),
            "https://github.com/FallenAngel97/tree-sitter-rego".into(),
            "https://github.com/stsewd/tree-sitter-rst".into(),
            "https://github.com/r-lib/tree-sitter-r".into(),
            "https://github.com/tree-sitter/tree-sitter-ruby".into(),
            "https://github.com/tree-sitter/tree-sitter-rust".into(),
            "https://github.com/tree-sitter/tree-sitter-scala".into(),
            "https://github.com/6cdh/tree-sitter-scheme".into(),
            "https://github.com/serenadeai/tree-sitter-scss".into(),
            "https://github.com/AbstractMachinesLab/tree-sitter-sexp".into(),
            "https://github.com/amaanq/tree-sitter-smali".into(),
            "https://github.com/nilshelmig/tree-sitter-sourcepawn".into(),
            "https://github.com/BonaBeavis/tree-sitter-sparql".into(),
            "https://github.com/takegue/tree-sitter-sql-bigquery".into(),
            "https://github.com/m-novikov/tree-sitter-sql".into(),
            "https://github.com/dhcmrlchtdj/tree-sitter-sqlite".into(),
            "https://github.com/metio/tree-sitter-ssh-client-config".into(),
            "https://github.com/Himujjal/tree-sitter-svelte".into(),
            "https://github.com/alex-pinkus/tree-sitter-swift".into(),
            "https://github.com/SystemRDL/tree-sitter-systemrdl".into(),
            "https://github.com/duskmoon314/tree-sitter-thrift".into(),
            "https://github.com/ikatyang/tree-sitter-toml".into(),
            "https://github.com/nvim-treesitter/tree-sitter-query".into(),
            "https://github.com/BonaBeavis/tree-sitter-turtle".into(),
            "https://github.com/gbprod/tree-sitter-twig".into(),
            "https://github.com/tree-sitter/tree-sitter-typescript".into(),
            "https://github.com/tree-sitter/tree-sitter-verilog".into(),
            "https://github.com/alemuller/tree-sitter-vhdl".into(),
            "https://github.com/ikatyang/tree-sitter-vue".into(),
            "https://github.com/wasm-lsp/tree-sitter-wasm".into(),
            "https://github.com/mehmetoguzderin/tree-sitter-wgsl".into(),
            "https://github.com/ikatyang/tree-sitter-yaml".into(),
            "https://github.com/Hubro/tree-sitter-yang".into(),
            "https://github.com/maxxnino/tree-sitter-zig".into(),
        ];

        Ok(Self {
            parsers,
        })
    }

    pub fn try_load(parser_config_path: Option<PathBuf>, save_default_if_missing: bool) -> Result<ParserConfig, String> {
        let path = if let Some(path) = parser_config_path {
            path
        } else if let Some(default_config_dir) = get_default_config_dir() {
            default_config_dir
        } else {
            dbg!(dirs::config_dir());
            return Err(format!("Unable to determine default parser configuration path."));
        };

        // Save default or Load config from fs
        let path = if !path.is_file() {
            path.join(PARSERS_CONFIG_FILE)
        } else {
            path
        };
        if path.exists() {
            let config_str = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
            let config = serde_json::from_str(&config_str).map_err(|e| e.to_string())?;

            Ok(config)
        } else if save_default_if_missing {
            let config = ParserConfig::create_with_known()?;
            let config_str = serde_json::to_string(&config).map_err(|e| e.to_string())?;
            std::fs::write(path, config_str).map_err(|e| e.to_string())?;

            Ok(config)
        } else {
            return Err(format!("Unable to find parsers configuration at path: {}", path.display()));
        }
    }
}

impl Grammars {
    pub fn load(parser_config_path: Option<PathBuf>, save_default_if_missing: bool) -> Result<Self, String> {
        let ts_config = if let Some(path) = parser_config_path.clone().or_else(|| get_default_config_dir()) {
            let ts_config_path = path.join(TREE_SITTER_CONFIG_FILE);
            if ts_config_path.exists() {
                let ts_config_str = std::fs::read_to_string(ts_config_path).map_err(|e| e.to_string())?;
                let ts_config: Config = serde_json::from_str(&ts_config_str).map_err(|e| e.to_string())?;

                ts_config
            } else if save_default_if_missing {
                let default_parsers_path = get_default_parsers_dir().expect("Unable to determine default parsers path!");
                let mut ts_config = Config::initial();
                ts_config.parser_directories.push(default_parsers_path);

                ts_config
            } else {
                Config::initial()
            }
        } else {
            Config::initial()
        };
        let parser_config = ParserConfig::try_load(parser_config_path, save_default_if_missing)?;

        let mut loader = Loader::new().map_err(|e| e.to_string())?;
        loader.find_all_languages(&ts_config).map_err(|e| e.to_string())?;

        Ok(Self {
            loader,
            ts_config,
            parser_config,
        })
    }

    pub fn try_get_language(&self, path: &Path) -> Result<Option<Language>, String> {
        match self.loader.language_configuration_for_file_name(path).map_err(|e| e.to_string())? {
            Some((lang, _)) => Ok(Some(lang)),
            None => Ok(None),
        }
    }


    pub fn get_configured_paths(&self) -> Vec<&str> {
        let mut paths = Vec::new();
        for dir in self.ts_config.parser_directories.iter() {
            if let Some(dir) = dir.to_str() {
                paths.push(dir)
            }
        }
        paths
    }

    pub fn try_install_languages(&self) -> Result<(), String> {
        fn clone_repo_in_dir(url: &str, dir: &PathBuf) -> Result<(), String> {
            let output = std::process::Command::new("git")
                .arg("clone")
                .arg(url)
                .arg(dir)
                .output()
                .expect("Failed to execute git command");

            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            let stdout = String::from_utf8_lossy(&output.stdout).to_string();

            if output.status.success() {
                Ok(())
            } else if !stderr.is_empty() {
                Err(format!("{}", stderr))
            } else {
                Err(format!("Unable to execute git clone command: {}", stdout))
            }
        }
        for parser_url in self.parser_config.parsers.iter() {
            let repo_path = try_get_parser_repo_path(parser_url)?;
            dbg!(&repo_path);
            if !repo_path.exists() {
                clone_repo_in_dir(&parser_url, &repo_path)?;
            }
        }

        Ok(())
    }
}
