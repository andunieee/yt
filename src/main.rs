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

impl fmt::Display for Video {
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
