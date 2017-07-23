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
    error_chain! { }
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
    series: bool
}

fn get_page(url: &str) -> Result<String> {
    let mut resp = reqwest::get(url).chain_err(|| format!("Unable to retrieve URL {}", url))?;
    let mut result = String::new();
    resp.read_to_string(&mut result).chain_err(|| "Error reading response")?;
    Ok(result)
}

fn get_contents(doc: &Html, seen: &HashSet<String>, series: bool) -> Result<(String, Option<String>)> {
    let story_selector = Selector::parse(".b-story-body-x").unwrap();
    let page = doc.select(&story_selector)
        .next()
        .ok_or("Page had no story block")?
        .inner_html();
    let next_page_selector = Selector::parse(".b-pager-next").unwrap();
    let series_selector = Selector::parse("#b-series a.ser_link").unwrap();
    let next_page = doc.select(&next_page_selector).next()
        .and_then(|x| x.value().attr("href").map(|y| y.to_string()))
        .or({
            if series {
                doc.select(&series_selector)
                    .flat_map(|x| x.value().attr("href").map(|y| y.to_string()))
                    .find(|x| !seen.contains(x))
            } else {
                None
            }
        });
    
    Ok((page, next_page))
}

fn crawl(initial_url: String, series: bool) -> Result<String> {
    let mut next_page = Some(initial_url);
    let mut output_buf = String::new();
    let mut seen = HashSet::new();
    while let Some(next) = next_page {
        seen.insert(next.clone());
        print!("{reset}[ ] {next}", reset=color::Fg(color::Reset), next=next);
        let contents = get_page(next.as_str())?;
        let document = Html::parse_document(contents.as_str());
        let (contents, _next_page) = get_contents(&document, &seen, series)?;
        next_page = _next_page;
        output_buf.push_str(contents.as_str());
        println!("\r{right}{green}X",
            right=cursor::Right(1), green=color::Fg(color::Green));
    }
    Ok(output_buf)
}

fn run() -> Result<()> {
    let args = Args::from_args();
    let contents = crawl(args.initial_url, args.series)?;
    if let Some(output_fname) = args.output {
        let path = PathBuf::from(output_fname);
        let mut f = File::create(path).chain_err(|| "Error creating file")?;
        f.write_all(contents.as_bytes()).chain_err(|| "Error writing contents to file")?;
    } else {
        println!("{}", contents);
    }
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
