use std::{
    fs::{File, OpenOptions},
    io::{self, BufReader, BufWriter},
    path::PathBuf,
};

use clap::{Parser, Subcommand};
use reqwest::blocking::multipart;

// const BASE_URL: &str = "https://projectsrhz8elrg-filetransfer.functions.fnc.fr-par.scw.cloud/";
const BASE_URL: &str = "http://localhost:8080/";

#[derive(Debug, Parser)]
struct Args {
    #[arg(short, long, default_value_t = false)]
    verbose: bool,

    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    Download { key: String, dest: PathBuf },
    Upload { key: String, src: PathBuf },
}

fn main() {
    let args = Args::parse();

    match args.command {
        Command::Download { key, dest } => {
            let writer = OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(true)
                .open(&dest)
                .expect("cannot open destination file");
            let mut writer = BufWriter::new(writer);

            if args.verbose {
                eprintln!("-- file {} opened for writing", dest.display());
                eprintln!("-- sending download request for key '{key}'");
            }

            let mut resp = reqwest::blocking::get(format!("{BASE_URL}{key}"))
                .expect("cannot download file")
                .error_for_status()
                .expect("server returned an error");

            if args.verbose {
                eprintln!("-- downloading file");
            }

            io::copy(&mut resp, &mut writer).expect("cannot write to file");

            if args.verbose {
                eprintln!("-- download finished");
            }
        }
        Command::Upload { key, src } => {
            let reader = File::open(&src).expect("cannot open source file");
            let reader = BufReader::new(reader);

            if args.verbose {
                eprintln!("-- file {} opened for reading", src.display());
                eprintln!("-- sending upload request for key '{key}'");
            }

            let form = multipart::Form::new().part("file", multipart::Part::reader(reader));

            let client = reqwest::blocking::Client::new();
            let _resp = client
                .post(format!("{BASE_URL}{key}"))
                .multipart(form)
                .send()
                .expect("cannot upload file")
                .error_for_status()
                .expect("server returned an error");

            if args.verbose {
                eprintln!("-- upload finished");
            }
        }
    };
}
