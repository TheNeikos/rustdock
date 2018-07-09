extern crate clap;
#[macro_use] extern crate serde_derive;
extern crate toml;
extern crate xdg;
extern crate textwidth;
extern crate regex;


mod config;

use std::io::Write;
use std::process::{Command, Stdio};
use std::sync::mpsc;

use clap::{Arg, App};

use config::{get_config, Element};

fn main() {
    let matches = App::new("RustDock")
        .version("1.0")
        .author("Marcel Müller <neikos@neikos.email>")
        .about("Displays a bar with content chosen at start time.")
        .arg(Arg::with_name("height")
                .short("h")
                .long("height")
                .value_name("SIZE")
                .takes_value(true))
        .arg(Arg::with_name("width")
                .short("w")
                .long("width")
                .value_name("SIZE")
                .takes_value(true))
        .arg(Arg::with_name("xpos")
                .short("x")
                .long("x-position")
                .value_name("X-COORDINATE")
                .takes_value(true))
        .arg(Arg::with_name("ypos")
                .short("y")
                .long("y-position")
                .value_name("Y-COORDINATE")
                .takes_value(true))
        .arg(Arg::with_name("config")
                .short("c")
                .long("config")
                .value_name("FILE")
                .takes_value(true))
        .get_matches();

    let config = get_config(matches);

    let mut child = Command::new("dzen2")
        .arg("-w").arg(config.dimensions.width.to_string())
        .arg("-h").arg(config.dimensions.height.to_string())
        .arg("-x").arg(config.dimensions.x.to_string())
        .arg("-y").arg(config.dimensions.y.to_string())
        .arg("-fn").arg(config.font.clone())
        .stdin(Stdio::piped())
        .spawn()
        .expect("Failed to start dzen2");

    let stdin = child.stdin.as_mut().expect("failed to get stdin");

    let re = regex::Regex::new(r"\^\w+\([#\w+]*\)").unwrap();

    let mut threads = vec![];
    let (tx, rx) = mpsc::channel();
    let mut data = vec![(String::new(), 0); config.elements.len()];
    for (element, idx) in config.elements.iter().zip(0..) {
        use std::thread;
        use std::process::Stdio;
        use std::env;
        use std::io::{BufReader, BufRead, Read};
        use std::time::{Duration, Instant};

        let elem = element.clone();
        let tx = tx.clone();
        let name = match &elem {
            Element::Command { .. } => {
                String::from("rustdock - command")
            }
            _ => {
                String::from("rustdock - unknown child")
            }
        };
        let handle = thread::Builder::new().name(name).spawn(move || {
            match elem {
                Element::Command { command, .. } => {
                    let shell = env::var("SHELL").unwrap_or("/bin/sh".to_string());
                    let child = Command::new(shell)
                        .args(&["-c", &command])
                        .stdout(Stdio::piped())
                        .stderr(Stdio::piped())
                        .spawn().expect("Could not spawn command");

                    let reader = BufReader::new({
                        if let Some(out) = child.stdout {
                            out
                        } else {
                            return;
                        }
                    });

                    for line in reader.lines() {
                        let line = line.expect("Read from process failed");
                        tx.send((idx, line)).expect("Could not send to main thread");
                    }
                }
                Element::Repeat { command, time, .. } => {
                    let shell = env::var("SHELL").unwrap_or("/bin/sh".to_string());
                    loop {
                        let start = Instant::now();
                        let child = Command::new(shell.clone())
                            .args(&["-c", &command])
                            .stdout(Stdio::piped())
                            .stderr(Stdio::piped())
                            .spawn().expect("Could not spawn command");

                        let mut line = String::new();
                        child.stdout.expect("Could not get stdout from child")
                            .read_to_string(&mut line).expect("Could not read from process");

                        let line = line.trim_matches('\n').trim_matches('\r').to_string();

                        tx.send((idx, line)).expect("Could not send to main thread");
                        let time = Duration::from_millis(time as u64) - start.elapsed();
                        thread::sleep(time);
                    }
                }
                Element::Fixed { size } => {
                    tx.send((idx, " ".repeat(size as usize))).unwrap();
                }
                Element::Seperator { sep } => {
                    tx.send((idx, sep.clone())).unwrap();
                }
                Element::Right => {
                    // Do nothing
                }
            }
        });
        threads.push(handle);
    }

    let context = textwidth::Context::new(&config.font).expect("could not initiate font context");

    let mut update_data = |data: &mut Vec<(String, u32)>| {
        let max_length = config.dimensions.width;

        let mut left_size : u32 = 0;
        let mut right_size : u32 = 0;
        let mut left_string = String::new();
        let mut right_string = String::new();

        {
            let mut last_index = config.elements.len();
            for (elem, idx) in config.elements.iter().zip(0..) {
                let (ref result, ref length) = data[idx];
                if let Element::Right = elem {
                    last_index = idx + 1;
                    break;
                }
                left_string.push_str(&result);
                left_size += length;
            }
            for (elem, idx) in config.elements[last_index..].iter().zip(last_index..) {
                let (ref result, ref length) = data[idx];
                right_string.push_str(&result);
                right_size += length;
            }
        }

        if left_size + right_size > max_length {
            left_size = (max_length - right_size).max(0);
            left_string.truncate(left_size as usize + 1);
            left_string.push('…')
        }
        let spacing = max_length as usize - right_size as usize;

        stdin.write_all(format!(
                "{}^pa({}){}\n",
                left_string,
                spacing,
                right_string
                ).as_bytes()).expect("failed to write to stdin");
    };

    for (idx, val) in rx.iter() {
        let width = if let Some(width) = config.elements[idx].get_width() {
            width
        } else {
            let only_text = re.replace_all(&val, "");
            textwidth::get_text_width(&context, &only_text) as u32
        };
        data[idx] = (val, width);
        update_data(&mut data);
    }
}
