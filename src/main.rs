/// anonbattle
///
/// A program to anonymize Pokémon Showdown battle logs
///
/// Written by Annika

mod anonymizer;
use anonymizer::*;

use std::fs;
use std::path::PathBuf;
use structopt::StructOpt;

#[derive(StructOpt)]
#[structopt(rename_all = "kebab-case")]
struct Options {
    #[structopt(short = "i", long = "input")]
    #[structopt(parse(from_os_str))]
    inputs: Vec<PathBuf>,

    #[structopt(short = "o", long = "output")]
    #[structopt(parse(from_os_str))]
    output_dir: PathBuf,

    #[structopt(short = "f", long = "format")]
    format: String,
}

fn handle_dir(in_dir: PathBuf, out_dir: PathBuf, format: &str, anonymizer: &mut Anonymizer) -> std::io::Result<()> {
    println!("Anonymizing {}...", in_dir.to_str().unwrap());
    for entry in fs::read_dir(in_dir)? {
        let path = entry?.path();
        if path.is_dir() {
            let dir_name = path.file_name().unwrap().to_str().unwrap();
            if dir_name.starts_with("gen") && dir_name != format {
                // It's for a different format
                continue;
            }
            handle_dir(path, out_dir.clone(), format, anonymizer)?;
        } else {
            // sanity check uwu
            if !path.to_str().unwrap().contains(format) {
                continue;
            }

            let (anonymized, number) = anonymizer.anonymize(&fs::read_to_string(path)?);
            let mut out_path = out_dir.clone();
            out_path.push(PathBuf::from(format!("battle-{}-{}.log.json", format, number)));
            fs::write(out_path, anonymized)?;
        }
    }
    Ok(())
}

fn main() -> std::io::Result<()> {
    let options = Options::from_args();
    let mut anon = Anonymizer::new();

    fs::create_dir(options.output_dir.clone()).unwrap_or(());

    for dir in options.inputs {
        handle_dir(dir, options.output_dir.clone(), &options.format, &mut anon).unwrap();
    }

    Ok(())
}
