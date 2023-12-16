use serde::Deserialize;

fn main() {
    seahorse::App::new("youtube")
        .action(run)
        .run(std::env::args().collect())
}

fn run(c: &seahorse::Context) {
    let q = c.args.first().map_or("".to_string(), |str| str.to_owned());
    let base = get_random_instance();

    let res = tinyget::get(format!("{}/api/v1/search/?q={}", base, q))
        .send()
        .expect(format!("failed to search {}", &base).as_str());

    let results: Vec<Video> =
        serde_json::from_slice(res.as_bytes()).expect("failed to deserialize video results");

    for vid in results {
        if vid.type_ != "video" {
            continue;
        }

        println!("{}: {} / {}", vid.title, vid.id, vid.view_count_text);
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
    title: String,
    #[serde(rename = "videoId")]
    id: String,
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

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct Thumb {
    quality: String,
    url: String,
    width: u16,
    height: u16,
}
