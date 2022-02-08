use std::fs::File;
use std::io::prelude::*;

use reqwest::header::{HeaderName, HeaderValue};
use reqwest::{Client, Error, RequestBuilder};
use serde_json::{Map, Value};

type SubmissionEntry = Map<String, Value>;

const HEADER: &str = r#"authority: leetcode.com
sec-ch-ua: " Not;A Brand";v="99", "Google Chrome";v="97", "Chromium";v="97"
accept: */*
x-newrelic-id: UAQDVFVRGwEAXVlbBAg=
x-requested-with: XMLHttpRequest
sec-ch-ua-mobile: ?0
user-agent: Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/97.0.4692.99 Safari/537.36
sec-ch-ua-platform: "Windows"
sec-fetch-site: same-origin
sec-fetch-mode: cors
sec-fetch-dest: empty
referer: https://leetcode.com/submissions/
accept-language: en-US,en;q=0.9,ko-KR;q=0.8,ko;q=0.7
cookie: NEW_PROBLEMLIST_PAGE=1; {FILL THE REST}
dnt: 1
sec-gpc: 1"#;

fn filter_submissions(val: &Value) -> Option<SubmissionEntry> {
    match val {
        Value::Object(x) => Some(x.clone()),
        _ => None,
    }
}

async fn get_submissions(req_builder: RequestBuilder) -> Result<Vec<SubmissionEntry>, Error> {
    let json_obj = req_builder
        .send()
        .await?
        .json::<Map<String, Value>>()
        .await?;
    let submissions = match json_obj["submissions_dump"].clone() {
        Value::Array(v) => v.iter().filter_map(|val| filter_submissions(val)).collect(),
        _ => {
            vec![]
        }
    };

    Ok(submissions)
}

async fn scrape_page(client: &Client, page_id: i32) -> Result<Vec<SubmissionEntry>, Error> {
    let mut req_builder = client.get(format!(
        "https://leetcode.com/api/submissions/?offset={}&limit=20&lastkey=",
        page_id * 20
    ));

    let lines: Vec<&str> = HEADER.lines().collect();
    for line in lines {
        let header: Vec<&str> = line.split_terminator(": ").collect();
        req_builder = req_builder.header(
            HeaderName::from_static(header[0]),
            HeaderValue::from_static(header[1]),
        );
    }
    let submissions: Result<Vec<SubmissionEntry>, Error> = get_submissions(req_builder).await;
    submissions
}

fn main() -> std::io::Result<()> {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let client = Client::new();

    let mut page_id: i32 = 15;
    while page_id < 23 {
        println!("Scraping page {}...", page_id + 1);
        let submissions: Vec<SubmissionEntry> = rt.block_on(scrape_page(&client, page_id)).unwrap();
        for s in &submissions {
            let file_path: String = format!(
                "scraped_leetcode_solutions/{}.cpp",
                match &s["title_slug"] {
                    Value::String(t) => {
                        t.trim_matches('"')
                    }
                    _ => {
                        unreachable!()
                    }
                }
            );
            if std::path::Path::new(&file_path).exists() || s["status_display"] != "Accepted" {
                continue;
            }

            let mut file = File::create(&file_path)?;
            let code: &str = match &s["code"] {
                Value::String(t) => t,
                _ => {
                    unreachable!()
                }
            };
            println!("Saving new solution at {}", file_path);
            file.write_all(code.as_bytes())?;
        }
        page_id = page_id + 1;
        std::thread::sleep(std::time::Duration::from_secs(1));
    }
    return Ok(());
}
