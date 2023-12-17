use inquire::Select;
use pad::PadStr;
use serde::Deserialize;
use std::fmt;
use std::os::unix::process::CommandExt;

fn main() {
    seahorse::App::new("youtube")
        .action(run)
        .run(std::env::args().collect())
}

fn run(c: &seahorse::Context) {
    let qf = c.args.join(" ");
    let q = qf.trim();
    if q == "" {
        return;
    }

    let base = get_random_instance();

    let res = tinyget::get(format!("{}/api/v1/search/?q={}", base, q))
        .send()
        .expect(format!("failed to search {}", &base).as_str());

    let results: Vec<Video> =
        serde_json::from_slice(res.as_bytes()).expect("failed to deserialize video results");

    let mut select = Select::new("Video results:", results);
    select.vim_mode = true;

    match select.prompt() {
        Err(_) => {}
        Ok(choice) => {
            println!(
                "chosen {}, opening {}",
                &choice.title.as_ref().unwrap_or(&"<no-title>".to_string()),
                &choice.id.as_ref().unwrap_or(&"<no-id>".to_string())
            );
            match choice.id {
                Some(id) => {
                    std::process::Command::new("mpv")
                        .arg(format!("https://youtube.com/watch?v={}", id))
                        .exec();
                }
                None => {}
            }
        }
    }
}

fn get_random_instance() -> String {
    let home = homedir::get_my_home().unwrap().unwrap();
    let cache_dir = home.join(".cache/yt");
    std::fs::create_dir_all(cache_dir).expect("failed to create cache dir");

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

    instances[rand::random::<usize>() % instances.len()].clone()
}

#[derive(Debug, Deserialize)]
struct Instance {
    #[serde(rename = "type")]
    type_: String,
    uri: String,
    api: Option<bool>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct Video {
    #[serde(rename = "type")]
    type_: String,
    title: Option<String>,
    #[serde(rename = "videoId")]
    id: Option<String>,
    author: String,
    #[serde(rename = "authorVerified")]
    verified: bool,
    #[serde(rename = "videoThumbnails")]
    thumbnails: Vec<Thumb>,
    description: String,
    #[serde(rename = "descriptionHtml")]
    description_html: String,
    #[serde(rename = "viewCount")]
    view_count: u32,
    #[serde(rename = "viewCountText")]
    view_count_text: String,
    #[serde(rename = "lengthSeconds")]
    length_seconds: u32,
    published: u32,
    #[serde(rename = "publishedText")]
    published_text: String,
}

impl fmt::Display for Video {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.type_.as_str() {
            "video" => {
                let duration = if self.length_seconds < 60 {
                    format!("{}s", self.length_seconds)
                } else {
                    format!(
                        "{}min{}s",
                        self.length_seconds / 60,
                        self.length_seconds % 60
                    )
                };

                write!(
                    f,
                    "{} | {} | {} | {} | {} | {}",
                    clamped(
                        &self.title.as_ref().unwrap_or(&"<no-title>".to_string()),
                        40
                    ),
                    clamped(&self.author, 14),
                    clamped(&duration, 6),
                    clamped(&self.view_count_text, 8),
                    clamped(&self.published_text, 11),
                    self.description
                )
            }
            _ => {
                write!(
                    f,
                    "[{}] {}",
                    self.type_,
                    &self.title.as_ref().unwrap_or(&"<no-title>".to_string()),
                )
            }
        }
    }
}

fn clamped(str: &String, pct: usize) -> String {
    let (cols, _) = crossterm::terminal::size().unwrap();
    let width = std::cmp::min(90, cols) as usize;
    let to = width * pct / 100;

    let mut v = str.pad_to_width(to);
    v.truncate(to);
    v
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct Thumb {
    quality: String,
    url: String,
    width: u16,
    height: u16,
}
