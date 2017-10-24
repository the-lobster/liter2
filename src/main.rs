#![recursion_limit = "1024"]
extern crate reqwest;
extern crate scraper;
extern crate structopt;
#[macro_use]
extern crate structopt_derive;
#[macro_use]
extern crate error_chain;
extern crate termion;

use structopt::StructOpt;
use std::collections::HashSet;
use std::path::PathBuf;
use std::io::{Read, Write};
use std::fs::File;
use scraper::{Html, Selector};
use termion::{color, cursor};

mod errors {
    error_chain!{}
}

use errors::*;

#[derive(StructOpt, Debug)]
#[structopt(name="liter2", about="Download entire stories from Literotica")]
struct Args {
    #[structopt(help="First page of the story to download")]
    initial_url: String,
    #[structopt(short="o", long="output", help="Name of file to write output")]
    output: Option<String>,
    #[structopt(long="series", help="Download entire series, vs a single story")]
    series: bool,
}

fn get_page(url: &str) -> Result<String> {
    let mut resp = reqwest::get(url).chain_err(|| format!("Unable to retrieve URL {}", url))?;
    let mut result = String::new();
    resp.read_to_string(&mut result).chain_err(|| "Error reading response")?;
    Ok(result)
}

fn get_next_chapter(doc: &Html, seen: &HashSet<String>) -> Result<Option<String>> {
    let series_selector = Selector::parse("#b-series a.ser_link").unwrap();

    Ok(doc.select(&series_selector)
        .flat_map(|x| x.value().attr("href").map(|y| y.to_string()))
        .find(|x| !seen.contains(x)))

}

fn get_contents(doc: &Html) -> Result<(String, Option<String>)> {
    let story_selector = Selector::parse(".b-story-body-x").unwrap();
    let page = doc.select(&story_selector)
        .next()
        .ok_or("Page had no story block")?
        .inner_html();
    let next_page_selector = Selector::parse(".b-pager-next").unwrap();
    let next_page = doc.select(&next_page_selector)
        .next()
        .and_then(|x| x.value().attr("href").map(|y| y.to_string()));

    Ok((page, next_page))
}

fn get_title(doc: &Html) -> Result<String> {
    let title_selector = Selector::parse(".b-story-header h1").unwrap();
    let title = doc.select(&title_selector).next().ok_or("Unable to find title")?;
    Ok(title.text().collect::<String>())
}


fn get_chapter(initial_url: &str) -> Result<(Option<String>, String, Option<Html>)> {
    let mut next_page = Some(initial_url.to_string());
    let mut output_buf = String::new();

    let mut first = true;
    let mut title = None;
    let mut last_doc = None;
    while let Some(next) = next_page {

        print!("{reset}[ ] {next}",
               reset = color::Fg(color::Reset),
               next = next);
        let contents = get_page(next.as_str())?;
        let document = Html::parse_document(contents.as_str());
        if first {
            title = Some(get_title(&document)?);
        }
        first = false;
        let (contents, _next_page) = get_contents(&document)?;
        next_page = _next_page;
        output_buf.push_str(contents.as_str());
        println!("\r{right}{green}X",
                 right = cursor::Right(1),
                 green = color::Fg(color::Green));
        last_doc = Some(document);
    }
    Ok((title, output_buf, last_doc))
}

fn write_to_dest(contents: &str, dest: &Option<PathBuf>) -> Result<()> {
    if let &Some(ref path) = dest {
        let mut f = File::create(path).chain_err(|| "Error creating file")?;
        f.write_all(contents.as_bytes()).chain_err(|| "Error writing contents to file")?;
    } else {
        println!("{}", contents);
    }
    Ok(())
}

fn crawl(initial_url: String, series: bool, dest: Option<PathBuf>) -> Result<()> {
    if series {
        let mut seen = HashSet::new();
        if let &Some(ref output_dirname) = &dest {
            std::fs::create_dir(output_dirname).chain_err(|| "Could not create output directory")?;
        }
        let mut url = Some(initial_url.clone());
        while let Some(next_url) = url {
            seen.insert(next_url.clone());
            let (title, mut contents, doc) = get_chapter(next_url.as_ref())?;
            let doc = doc.unwrap();
            let mut title = title.or(initial_url.split("/").last().map(|x| x.to_string()))
                .unwrap_or("Unknown story".to_string());
            let mut heading = String::from("<h1>");
            heading.push_str(title.as_str());
            heading.push_str("</h1>");
            contents.insert_str(0, heading.as_str());
            url = get_next_chapter(&doc, &seen)?;
            title.push_str(".html");
            let updated_path = dest.clone().map(|x| {
                let mut y = x.clone();
                y.push(title);
                y
            });
            write_to_dest(&contents, &updated_path)?;
        }
    } else {
        let (_, contents, _) = get_chapter(initial_url.as_ref())?;
        // let title = title.or(initial_url.split("/").last().map(|x| x.to_string()))
        //     .unwrap_or("Unknown story".to_string());
        write_to_dest(&contents, &dest)?;
    }
    Ok(())
}

fn run() -> Result<()> {
    let args = Args::from_args();
    crawl(args.initial_url,
          args.series,
          args.output.map(PathBuf::from))?;
    Ok(())
}

fn main() {
    if let Err(ref e) = run() {
        println!("Error: {}", e);
        for e in e.iter().skip(1) {
            println!("caused by: {}", e)
        }

        if let Some(backtrace) = e.backtrace() {
            println!("Backtrace: {:?}", backtrace);
        }

        ::std::process::exit(1);
    }
}
