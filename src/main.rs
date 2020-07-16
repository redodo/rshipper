use std::fs::File;
use std::io::prelude::*;
use std::io::{self, Read};

use clap::Clap;
use image;

mod functions;

#[derive(Clap)]
struct Opts {
    #[clap(subcommand)]
    command: Command,
}

#[derive(Clap)]
enum Command {
    Load(Load),
    Unload(Unload),
    Expose(Expose),
    Debug(Debug),
}

#[derive(Clap)]
struct Load {
    #[clap(short, long)]
    input: Option<String>,
    container: String,
    #[clap(short, long)]
    output: String,
}

#[derive(Clap)]
struct Unload {
    container: String,
    #[clap(short, long)]
    output: Option<String>,
}

#[derive(Clap)]
struct Expose {
    source: String,
    destination: String,
}

#[derive(Clap)]
struct Debug {}

fn main() -> Result<(), String> {
    let opts = Opts::parse();

    match opts.command {
        Command::Load(c) => {
            // Get a password from the user
            let password = functions::prompt_password(true)?;

            // Read the input from stdin or file
            let mut input_data: Vec<u8> = Vec::new();
            match c.input {
                Some(filename) => {
                    let mut handle = File::open(filename).unwrap();
                    handle.read_to_end(&mut input_data).unwrap();
                }
                None => {
                    let stdin = io::stdin();
                    let mut handle = stdin.lock();
                    handle.read_to_end(&mut input_data).unwrap();
                }
            }

            println!("Loading image...");
            let img = image::open(c.container).unwrap();
            let rgb = img.to_rgb();
            let width = rgb.width();
            let height = rgb.height();
            let mut container = rgb.into_raw();

            functions::encode(&input_data, &password, &mut container)?;

            println!("Writing image...");
            let new = image::RgbImage::from_raw(width, height, container).unwrap();
            new.save(c.output).unwrap();
        }
        Command::Unload(c) => {
            let img = image::open(c.container).unwrap();
            let rgb = img.to_rgb();
            let mut container = rgb.into_raw();

            let password = functions::prompt_password(false)?;

            let payload = functions::decode(&password, &mut container);
            match c.output {
                Some(filename) => {
                    let mut handle = File::create(filename).unwrap();
                    handle.write_all(&payload[..]).unwrap();
                }
                None => {
                    let stdout = io::stdout();
                    let mut handle = stdout.lock();
                    handle.write_all(&payload[..]).unwrap();
                }
            }
        }
        Command::Expose(c) => {
            let img = image::open(c.source).unwrap();
            let mut rgb = img.to_rgb();
            for pixel in rgb.pixels_mut() {
                pixel[0] = (pixel[0] & 1) * 255;
                pixel[1] = (pixel[1] & 1) * 255;
                pixel[2] = (pixel[2] & 1) * 255;
            }
            rgb.save(c.destination).unwrap();
        }
        Command::Debug(_) => {}
    }
    Ok(())
}
