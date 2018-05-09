use date::Date;
use failure::{err_msg, Error};
use scraper::{Html, Selector, ElementRef};
use reqwest;
use termion::{color, cursor};
use std::path::PathBuf;
use std::collections::{HashSet, HashMap};
use epub_builder::{EpubBuilder, ZipLibrary, EpubContent, ReferenceType};

lazy_static! {
    static ref STORY_ROWS: Selector = Selector::parse(".sl, .r-ott").unwrap();
    static ref AP_TITLE: Selector = Selector::parse("td:nth-child(1) a").unwrap();
    static ref AP_DESC: Selector = Selector::parse("td:nth-child(2)").unwrap();
    static ref AP_DATE: Selector = Selector::parse("td:nth-child(4)").unwrap();
    static ref AP_NAME: Selector = Selector::parse("a.contactheader").unwrap();
    static ref AUTHOR_LINK: Selector = Selector::parse(".b-story-user-y > a").unwrap();
}

#[derive(Debug)]
struct Chapter<'title> {
    title: &'title Title,
    contents: String,
}

#[derive(Debug)]
pub struct Title {
    name: String,
    desc: String,
    url: String,
    date: Date,
}

#[derive(Debug)]
pub struct AuthorPage {
    name: String,
    titles: Vec<Title>,
}

fn get_author_page(page: &str) -> Result<String, Error> {
    let mut resp = reqwest::get(page)?;
    let content = resp.text()?;
    Ok(content)
}

fn text_from_selector(element: &ElementRef, sel: &Selector) -> Result<String, Error> {
    Ok(element.select(sel)
        .next()
        .expect("No such element?")
        .text()
        .map(|s| s.trim())
        .collect::<Vec<_>>()
        .join(""))
}

pub fn get_stories(page: &str) -> Result<AuthorPage, Error> {
    let html = get_author_page(page)?;
    let doc = Html::parse_document(html.as_str());
    let author = doc.select(&AP_NAME)
        .next()
        .ok_or(format_err!("No author on {}?", page))?
        .text()
        .next()
        .unwrap()
        .to_string();
    let rows: Result<Vec<_>, Error> = doc.select(&STORY_ROWS)
        .map(|row| {
            let name = text_from_selector(&row, &AP_TITLE)?;
            let url = row.select(&AP_TITLE)
                .next()
                .expect("URL")
                .value()
                .attr("href")
                .expect("No link?")
                .to_string();
            let desc = text_from_selector(&row, &AP_DESC)?;
            let date = text_from_selector(&row, &AP_DATE)?;
            let date = Date::parse_mdy(date.as_str()).unwrap();
            Ok(Title {
                name: name,
                desc: desc,
                url: url,
                date: date,
            })
        })
        .collect();
    Ok(AuthorPage {
        name: author,
        titles: rows?,
    })
}


pub fn get_story_page(url: &str) -> Result<String, Error> {
    let mut resp = reqwest::get(url).map_err(|_| format_err!("Unable to retrieve URL {}", url))?;
    let result = resp.text()?;
    Ok(result)
}

pub fn get_chapter(initial_url: &str) -> Result<(Option<String>, String, Option<Html>), Error> {
    let mut next_page = Some(initial_url.to_string());
    let mut output_buf = String::new();

    let mut first = true;
    let mut title = None;
    let mut last_doc = None;
    while let Some(next) = next_page {

        print!("{reset}[ ] {next}",
               reset = color::Fg(color::Reset),
               next = next);
        let contents = get_story_page(next.as_str())?;
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

fn get_contents(doc: &Html) -> Result<(String, Option<String>), Error> {
    let story_selector = Selector::parse(".b-story-body-x").unwrap();
    let page = doc.select(&story_selector)
        .next()
        .ok_or(err_msg("Page had no story block"))?
        .inner_html();
    let next_page_selector = Selector::parse(".b-pager-next").unwrap();
    let next_page = doc.select(&next_page_selector)
        .next()
        .and_then(|x| x.value().attr("href").map(|y| y.to_string()));

    Ok((page, next_page))
}

fn get_title(doc: &Html) -> Result<String, Error> {
    let title_selector = Selector::parse(".b-story-header h1").unwrap();
    let title = doc.select(&title_selector).next().ok_or(err_msg("Unable to find title"))?;
    Ok(title.text().collect::<String>())
}

fn get_author_link(doc: &Html) -> Result<String, Error> {
    Ok(doc.select(&AUTHOR_LINK)
        .next()
        .expect("No author link?")
        .value()
        .attr("href")
        .expect("No link?")
        .to_string())
}

pub fn crawl(initial_url: &str, series: bool, dest: &Option<PathBuf>) -> Result<(), Error> {
    let mut epub =
        EpubBuilder::new(ZipLibrary::new()).map_err(|_| err_msg("Could not build epub builder"))?;
    let first_page = {
        let mut resp = reqwest::get(initial_url)?;
        Html::parse_document(resp.text()?.as_str())
    };
    let author_link = get_author_link(&first_page)?;
    let author_stories = get_stories(&author_link)?;
    epub.metadata("author", author_stories.name).unwrap();
    epub.inline_toc();
    let mut story_map = HashMap::new();
    for story in &author_stories.titles {
        story_map.insert(story.url.as_str(), story);
    }
    if series {
        let mut seen = HashSet::new();
        // if let Some(ref output_dirname) = *dest {
        //     ::std::fs::create_dir(output_dirname)?;
        // }
        let mut url = Some(initial_url.to_string());
        while let Some(next_url) = url {
            seen.insert(next_url.clone());
            let (title, mut contents, doc) = get_chapter(next_url.as_ref())?;
            let story_entry = story_map[next_url.as_str()];
            
            let doc = doc.unwrap();
            let mut title = title.or_else(|| initial_url.split('/').last().map(|x| x.to_string()))
                .unwrap_or_else(|| "Unknown story".to_string());
            let mut heading = String::from("<h1>");
            heading.push_str(title.as_str());
            heading.push_str("</h1>");
            contents.insert_str(0, heading.as_str());

            let full_title = format!("{}\n{}", story_entry.name, story_entry.desc);
            url = get_next_chapter(&doc, &seen)?;
            title.push_str(".html");
            epub.add_content(EpubContent::new(title.as_str(), contents.as_bytes())
                .title(full_title.as_str())
                .reftype(ReferenceType::Text));
            let updated_path = dest.clone().map(|x| {
                let mut y = x.clone();
                y.push(title);
                y
            });
            // ::write_to_dest(&contents, &updated_path)?;
        }
    } else {
        let (_, contents, _) = get_chapter(initial_url)?;
        let story_entry = story_map[initial_url];
        let full_title = format!("{}\n{}", story_entry.name, story_entry.desc);
        epub.add_content(EpubContent::new("", contents.as_bytes())
                .title(full_title.as_str())
                .reftype(ReferenceType::Text));
        // let title = title.or(initial_url.split("/").last().map(|x| x.to_string()))
        // ::write_to_dest(&contents, dest)?;
    }
    match *dest {
        Some(ref dest) => {
            use std::fs::File;
            let f = File::create(dest)?;
            epub.generate(f).unwrap();
        },
        None => {
            
        }
    }
    
    Ok(())
}

fn get_next_chapter(doc: &Html, seen: &HashSet<String>) -> Result<Option<String>, Error> {
    let series_selector = Selector::parse("#b-series a.ser_link").unwrap();

    Ok(doc.select(&series_selector)
        .flat_map(|x| x.value().attr("href").map(|y| y.to_string()))
        .find(|x| !seen.contains(x)))

}
