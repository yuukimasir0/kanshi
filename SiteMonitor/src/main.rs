use chrono::Local;
use encoding_rs::SHIFT_JIS;
use reqwest::Error;
use select::document::Document;
use select::node::Node;
use select::predicate::Name;
use std::collections::HashMap;
use std::fs::File;
use std::io::{self, Write};
use tokio::time::{sleep, Duration};

async fn fetch_and_check_change(url: &str) -> Result<Vec<(String, String)>, Error> {
    let bytes = reqwest::get(url).await?.bytes().await?;
    let (cow, _, _) = SHIFT_JIS.decode(&bytes);
    let body = cow.into_owned();
    let document = Document::from(body.as_str());

    let all_rows_html = document
        .find(Name("TR"))
        .filter_map(|tr_node| extract_html(&tr_node))
        .collect::<Vec<(String, String)>>();
    Ok(all_rows_html)
}

fn extract_html(node: &Node) -> Option<(String, String)> {
    let product_code = node.find(Name("TD")).next()?.text().trim().to_string();
    let html_content = node.html();
    Some((product_code, html_content))
}

async fn make_html(name: &str, content: &str) -> io::Result<()> {
    let now = Local::now();
    let name = name.to_owned() + "_%Y%m%d_%H%M%S.html";
    let mut file = File::create(now.format(&name).to_string())?;
    writeln!(file, "{}", content)?;
    Ok(())
}

async fn check_website() {
    let url = "https://www.imon.co.jp/MODELS/INDEX171.MBR/LIST?T=";
    let mut previous_rows = HashMap::new();
    let mut check_s: u8 = 0;
    if let Ok(current_rows) = fetch_and_check_change(url).await {
        for (name, content) in current_rows {
            previous_rows.insert(name, (content, check_s));
        }
    }
    loop {
        if let Ok(current_rows) = fetch_and_check_change(url).await {
            for (name, current_content) in current_rows {
                match previous_rows.get_mut(&name) {
                    Some(previous_content) => {
                        if current_content != *previous_content.0 {
                            println!("変更が検出されました！");
                            let _ = make_html(&name, &current_content).await;
                            *previous_content = (current_content, check_s);
                        }
                    }
                    None => {
                        println!("追加されました！");
                        let _ = make_html(&name, &current_content).await;
                        previous_rows.insert(name, (current_content, check_s));
                    }
                }
            }
        }
        check_s += 1;
        check_s %= 128;
        if check_s == 0 {
            previous_rows.retain(|_, (_, s)| *s == 0);
        }
        sleep(Duration::from_secs(5)).await;
    }
}

#[tokio::main]
async fn main() {
    check_website().await;
}
