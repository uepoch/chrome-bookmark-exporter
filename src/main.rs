use anyhow::{anyhow, bail, Context, Result};
use std::{
    collections::HashMap, fs::read_to_string, io::stdout, borrow::Borrow,
};

use clap::Parser;
use serde::{Deserialize, Serialize};


#[cfg(target_os = "linux")]
static VARIANTS: &[&str] = &[
    "google-chrome",
    "google-chrome-beta",
    "google-chrome-unstable",
    "chromium",
];

#[cfg(not(target_os = "linux"))]
static VARIANTS: &[&str] = &[
    "Chrome",
    "Chrome Beta",
    "Chromium",
    #[cfg(target_os = "macos")]
    "Chrome Canary",
    #[cfg(target_os = "windows")]
    "Chrome SxS",
];

#[cfg(target_os = "windows")]
static DEFAULT_PATH: &str = r"%LOCALAPPDATA%\Google\{edition}\Default\Bookmarks";

#[cfg(target_os = "linux")]
static DEFAULT_PATH: &str = "$HOME/.config/{edition}/Default/Bookmarks";

#[cfg(target_os = "macos")]
static DEFAULT_PATH: &str = "$HOME/Library/Application Support/Google/{edition}/Default/Bookmarks";

#[derive(Parser)]
struct Args {
    #[arg(short, long, default_value = DEFAULT_PATH)]
    file: String,
    name: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct BookmarksFile {
    checksum: String,
    roots: HashMap<String, Bookmarks>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(untagged, rename_all = "lowercase")]
enum Bookmarks {
    Folder {
        name: String,
        children: Vec<Bookmarks>,
    },
    Url {
        name: String,
        url: String,
    },
}

fn find_folder(v: &[impl Borrow<Bookmarks>], needle: &str) -> Vec<Bookmarks> {
    v.iter().flat_map(|bk| match bk.borrow() {
        Bookmarks::Folder { name, children}  => {
            if name == needle {
                vec![bk.borrow().to_owned()]
            } else {
                find_folder(children, needle)
            }
        },
        _ => vec![]
    }).collect()
}

fn main() -> Result<()> {
    let args = Args::parse();

    let bookmark_file: BookmarksFile = VARIANTS
        .iter()
        .find_map(|variant| {
            let file = args.file.as_str().replace("{edition}", variant);
            let file = envmnt::expand(&file, None);
            eprintln!("trying {}..", file);
            let d = read_to_string(&file)
                .context(format!("reading {}", &file))
                .ok()?;
            eprintln!("found {}", file);
            Some(serde_json::from_str(&d).expect("failed to parse"))
        })
        .ok_or_else(|| anyhow!("couldn't find a valid file"))?;

    let bookmarks: Vec<Bookmarks> = bookmark_file.roots.into_values().collect();
    let selected = find_folder(&bookmarks, &args.name);
    if selected.is_empty() {
        bail!("couldn't find any folder named {}", args.name)
    }

    serde_json::to_writer_pretty(stdout(), &selected)?;
    Ok(())
}
