use inquire::{Confirm, Select};
use pad::PadStr;
use serde::Deserialize;
use std::{
    env, fmt,
    fs::{self, File},
    os::unix::process::CommandExt,
    path::Path,
    process::Command,
    time::{Duration, SystemTime},
};

fn main() {
    seahorse::App::new("yt")
        .usage("yt [search terms]")
        .description("search, select and play youtube videos")
        .action(run)
        .run(env::args().collect())
}

fn run(c: &seahorse::Context) {
    let qf = c
        .args
        .clone()
        .iter_mut()
        .map(|arg| arg.replace(" ", "+"))
        .collect::<Vec<String>>()
        .join("+");
    let q = qf.trim();
    if q == "" {
        return;
    }

    let base = get_random_instance();

    print!("searching {}", q);
    let res = tinyget::get(format!("{}/api/v1/search/?q={}", base, q))
        .send()
        .expect(format!("failed to search {}", &base).as_str());

    let mut viuer_config = viuer::Config::default();
    viuer_config.absolute_offset = false;
    viuer_config.restore_cursor = false;
    viuer_config.x = 4;
    viuer_config.transparent = true;

    loop {
        let results: Vec<SearchResult> = serde_json::from_slice(res.as_bytes()).expect(
            format!(
                "failed to deserialize video results: {}",
                res.as_str().expect("result-no-str")
            )
            .as_str(),
        );
        let mut select = Select::new("Video results:", results);
        select.vim_mode = true;

        match select.prompt() {
            Err(_) => {
                return;
            }
            Ok(choice) => match &choice.id.as_ref() {
                Some(id) => {
                    let arg = if choice.type_ == "video" {
                        let _ = choice.thumbnails.as_ref().map_or(None, |thumbs| {
                            let thumb = thumbs.iter().find(|thumb| thumb.quality == "default")?;
                            let res = tinyget::get(&thumb.url).send().ok()?;
                            let img = image::load_from_memory(res.as_bytes()).ok()?;
                            let smaller = viuer::resize(&img, Some(30), None);
                            viuer::print(&smaller, &viuer_config).ok()?;
                            Some(())
                        });

                        format!("https://youtube.com/watch?v={}", id)
                    } else if choice.type_ == "playlist" {
                        format!("https://youtube.com/playlist?list={}", id)
                    } else {
                        return;
                    };

                    println!("{}", choice);
                    match Confirm::new("continue?")
                        .with_default(true)
                        .with_parser(&|_| Ok(true))
                        .with_formatter(&|_| "".to_string())
                        .with_default_value_formatter(&|_| "".to_string())
                        .with_placeholder("")
                        .with_help_message("Enter to proceed, Esc to go back")
                        .prompt_skippable()
                        .ok()
                    {
                        Some(Some(true)) => {
                            Command::new("mpv").arg(arg).exec();
                            return;
                        }
                        _ => continue,
                    }
                }
                None => {
                    return;
                }
            },
        }
    }
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct SearchResult {
    #[serde(rename = "type")]
    type_: String,
    title: Option<String>,
    #[serde(rename = "videoId", alias = "playlistId")]
    id: Option<String>,
    author: Option<String>,
    #[serde(rename = "videoThumbnails")]
    thumbnails: Option<Vec<Thumb>>,
    description: Option<String>,
    #[serde(rename = "viewCountText")]
    view_count_text: Option<String>,
    #[serde(rename = "lengthSeconds")]
    length_seconds: Option<u32>,
    #[serde(rename = "publishedText")]
    published_text: Option<String>,
}

impl fmt::Display for SearchResult {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.type_.as_str() {
            "video" => {
                let duration = self.length_seconds.map_or("".to_string(), |ls| {
                    if ls < 60 {
                        format!("{}s", ls)
                    } else if ls < 60 * 20 {
                        format!("{}min{}s", ls / 60, ls % 60)
                    } else if ls < 60 * 60 {
                        format!("{}min", ls / 60)
                    } else {
                        format!("{}h{}min", ls / 3600, ls % 3600 / 60)
                    }
                });

                write!(
                    f,
                    "{} | {} | {} | {} | {} | {}",
                    clamped(
                        &self.title.as_ref().map_or("<no-title>", |s| s.as_str()),
                        60
                    ),
                    clamped(&duration, 7),
                    clamped(
                        &self
                            .view_count_text
                            .as_ref()
                            .map_or("", |s| s.as_str())
                            .split(" ")
                            .take(1)
                            .collect::<String>()
                            .as_str(),
                        5
                    ),
                    clamped(&self.published_text.as_ref().unwrap_or(&"_".to_string()), 8),
                    clamped(
                        &self.author.as_ref().map_or("<no-author>", |s| s.as_str()),
                        15
                    ),
                    &self.description.as_ref().map_or("", |s| s.as_str())
                )
            }
            "playlist" => {
                write!(
                    f,
                    "<playlist> \"{}\" by {}",
                    &self.title.as_ref().unwrap_or(&"<no-title>".to_string()),
                    clamped(
                        &self.author.as_ref().map_or("<no-author>", |s| s.as_str()),
                        15
                    ),
                )
            }
            _ => {
                write!(
                    f,
                    "<{}> {}",
                    self.type_,
                    &self.title.as_ref().unwrap_or(&"<no-title>".to_string()),
                )
            }
        }
    }
}

fn clamped(str: &str, pct: usize) -> String {
    let (cols, _) = crossterm::terminal::size().unwrap();
    let width = std::cmp::min(90, cols) as usize;
    let to = width * pct / 100;

    first_n_chars(str, to).pad_to_width(to)
}

fn first_n_chars(s: &str, n: usize) -> &str {
    if let Some((x, _)) = s.char_indices().nth(n) {
        &s[..x]
    } else {
        s
    }
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct Thumb {
    quality: String,
    url: String,
    width: u16,
    height: u16,
}

fn get_random_instance() -> String {
    let home = homedir::get_my_home().unwrap().unwrap();
    let mut cache_dir = home.join(".cache/yt");
    fs::create_dir_all(&cache_dir).expect("failed to create cache dir");

    cache_dir.push("invidious_instances.json");
    let cache_file_path = cache_dir.as_path();

    let instances = if !Path::new(cache_file_path).exists() {
        // file doesn't exist, fetch instances and save them locally
        let instances = fetch_instances_list();
        let file = File::create(cache_file_path).unwrap();
        serde_json::to_writer(file, &instances).unwrap();
        instances
    } else {
        // file exists, check its date
        let metadata = fs::metadata(cache_file_path).unwrap();
        let creation_time = metadata.created().unwrap();
        let now = SystemTime::now();
        let file_age = now.duration_since(creation_time).unwrap();
        if file_age > Duration::from_secs(14 * 24 * 60 * 60) {
            // it's too old, replace
            let instances = fetch_instances_list();
            let file = File::create(cache_file_path).unwrap();
            serde_json::to_writer(file, &instances).unwrap();
            instances
        } else {
            let file = File::open(cache_file_path).unwrap();
            serde_json::from_reader(file).expect("cached invidious list is broken")
        }
    };

    instances[rand::random::<usize>() % instances.len()].clone()
}

fn fetch_instances_list() -> Vec<String> {
    let res = tinyget::get("https://api.invidious.io/instances.json")
        .send()
        .expect("failed to get list of instances");
    let instances: Vec<String> = serde_json::from_slice::<Vec<(String, Instance)>>(res.as_bytes())
        .expect("failed to deserialize list of instances")
        .iter()
        .filter_map(|(_, inst)| {
            if inst.type_.starts_with("http") && Some(true) == inst.api {
                Some(inst.uri.clone())
            } else {
                None
            }
        })
        .collect();

    instances
}

#[derive(Debug, Deserialize)]
struct Instance {
    #[serde(rename = "type")]
    type_: String,
    uri: String,
    api: Option<bool>,
}
