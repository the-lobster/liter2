#![recursion_limit = "1024"]
extern crate reqwest;
extern crate scraper;
extern crate structopt;
#[macro_use]
extern crate structopt_derive;
#[macro_use]
extern crate failure;
extern crate termion;
extern crate epub_builder;
#[macro_use]
extern crate lazy_static;

mod liter;
mod date;


use failure::{err_msg, Error};
use structopt::StructOpt;
use std::path::PathBuf;
use std::io::Write;
use std::fs::File;

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

fn run() -> Result<(), Error> {
    let args = Args::from_args();
    liter::crawl(args.initial_url.as_ref(),
                 args.series,
                 &args.output.map(PathBuf::from))?;
    Ok(())
}

fn main() {
    if let Err(ref e) = run() {
        eprintln!("Error: {}", e);

        ::std::process::exit(1);
    }
}
