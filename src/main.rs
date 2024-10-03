// src/main.rs
mod book;
mod epub;
mod ffmpeg;
mod metdata;
use book::{get_titles, read_sections, Book}; // Import book module functions
use clap::Parser;
use colored::*;
use edge_tts::{build_ssml, request_audio};
use ffmpeg::concatenate_audio_files;
use indicatif::{ProgressBar, ProgressStyle};
use std::fs::{self, OpenOptions};
use std::io;
use std::io::Write;
use std::path::{Path, PathBuf};
use tokio::task;

const AUDIO_OUTPUT_DIR: &str = "./tmp"; // Set the output / temp directory

async fn read_chapter(chapter_number: usize, texts: Vec<String>) {
    if texts.len() < 2 {
        println!("Not enough text to display for chapter {}", chapter_number);
        return; // Early exit if there aren't enough texts
    } else {
        if let Some(first_line) = texts.get(0) {
            println!("{}", first_line.green());
        }

        // Print the rest in dark grey (or black)
        for line in &texts[1..4] {
            // Adjust the range as needed
            println!("{}", line.bright_black()); // You can also use line.black() for black color
        }
    }

    let mut tasks = Vec::new();
    let pb = ProgressBar::new(texts.len() as u64);
    let sty = ProgressStyle::with_template(
        "{spinner:.green} {msg} [{elapsed_precise}] {bar:40.cyan/blue} {pos:>7}/{len:7}",
    )
    .unwrap()
    .progress_chars("█░");
    pb.set_style(sty);

    for (i, text) in texts.iter().enumerate() {
        // Use chapter_number in the filename for unique identification
        let output_file = format!("{}/c{}_p_{}.mp3", AUDIO_OUTPUT_DIR, chapter_number, i + 1);

        let task = task::spawn({
            let text_clone = text.clone();
            let text_preview = text.clone();
            let output_file_clone = output_file.clone();
            let pb_clone = pb.clone(); // Clone the ProgressBar for use in the async block

            async move {
                if let Err(e) = gen_audio(text_clone, output_file_clone).await {
                    println!("Error generating audio {}", e);
                } else {
                    match fs::metadata(output_file.clone()) {
                        Ok(metadata) => {
                            let file_size = metadata.len(); // File size in bytes
                            if file_size < 1 {
                                println!("Empty File delting ({})", text_preview.black());
                                fs::remove_file(output_file.clone())
                                    .expect("Failed to remove file");
                            }
                        }
                        Err(e) => {
                            eprintln!("Error reading file metadata: {}", e);
                        }
                    }
                    pb_clone.inc(1); // Increment the progress bar
                }
            }
        });

        tasks.push(task);
    }

    for task in tasks {
        let _ = task.await; // Await each task
    }

    pb.finish_with_message("All audio files generated!"); // Finish the progress bar
}

async fn combine_chapter(mut files: Vec<String>, output_file: &str) {
    files.sort_by_key(|file| {
        let parts: Vec<&str> = file.split('_').collect();

        // Check if parts has at least 3 elements
        if parts.len() < 3 {
            eprintln!("Warning: Filename '{}' does not have enough parts", file);
            return 0; // or handle it in a way that makes sense for your application
        }

        // Try to parse the part as a number; log an error if it fails
        match parts[2].replace(".mp3", "").parse::<u32>() {
            Ok(num) => num,
            Err(_) => {
                eprintln!("Warning: Unable to parse number from '{}'", parts[2]);
                0 // Default value if parsing fails
            }
        }
    });
    if Path::new(output_file).exists() {
        println!("{output_file} already exists");
    } else {
        concatenate_audio_files(files, output_file); // Ensure you await the async function
    }
}

async fn gen_audio(txt: String, output_file: String) -> Result<(), Box<dyn std::error::Error>> {
    let audio_data = request_audio(
        &build_ssml(&txt, "en-US-BrianNeural", "medium", "medium", "medium"),
        "audio-24khz-96kbitrate-mono-mp3",
    )?;

    OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(&output_file)?
        .write_all(&audio_data)?;

    Ok(())
}

fn get_files(dir: &Path) -> io::Result<Vec<String>> {
    let mut files = Vec::new(); // Initialize a vector to store file paths

    // Iterate over entries in the specified directory
    for entry in fs::read_dir(dir)? {
        let entry = entry?; // Handle potential errors when accessing entries
        let path = entry.path(); // Get the path of the entry

        // Check if the entry is a file and ends with .mp3
        if path.is_file() && path.extension().map_or(false, |ext| ext == "mp3") {
            // Push the path as a String into the vector
            files.push(path.to_string_lossy().to_string());
        }
    }

    Ok(files) // Return the vector of file paths
}

fn get_chapter_number(entry: &str) -> Option<u32> {
    if let Some(pos) = entry.find("chapter_") {
        let number_part = &entry[pos + 8..];
        if let Some(m4a_pos) = number_part.find(".m4a") {
            return number_part[..m4a_pos].parse::<u32>().ok();
        }
    }
    None
}

fn get_chap_files(dir: &Path) -> io::Result<Vec<String>> {
    let mut files = Vec::new(); // Initialize a vector to store file paths

    // Iterate over entries in the specified directory
    for entry in fs::read_dir(dir)? {
        let entry = entry?; // Handle potential errors when accessing entries
        let path = entry.path(); // Get the path of the entry

        // Check if the entry is a file and ends with .mp3
        if path.is_file() && path.extension().map_or(false, |ext| ext == "m4a") {
            // Push the path as a String into the vector
            files.push(path.to_string_lossy().to_string());
        }
    }
    files.sort();

    Ok(files) // Return the vector of file paths
}
async fn make_book(book_path: &str, opf_file: &str, cover: &str) {
    let chapters = read_sections(book_path);
    let titles = get_titles(book_path);
    let min_length = chapters.len().min(titles.len());
    let mut chapter_lengths = Vec::new();

    let mut book = Book::new();
    if chapters[0][0].starts_with("Title: ") {
        // make work with python generated files
        println!("{}","using Python generated style file".yellow()); // informative
        for i in 1..min_length {
            
            book.add_chapter(&titles[i], chapters[i].clone());
        }
    } else {
        for i in 0..min_length {
            // work with rust generated files
            book.add_chapter(&titles[i], chapters[i].clone());
        }
    }

    for (chapter_number, (_, content)) in book.get_all_chapters().iter().enumerate() {
        if !Path::new(&format!(
            "{}/chapter_{}.m4a",
            AUDIO_OUTPUT_DIR, chapter_number
        ))
        .exists()
        {
            read_chapter(chapter_number + 1, content.to_vec()).await; // Pass chapter number
        } else {
            println!("Chapter already processed");
        }

        let mut file_paths = Vec::new();
        let dir = Path::new(AUDIO_OUTPUT_DIR);
        match get_files(dir) {
            Ok(files) => {
                file_paths = files;
            }
            Err(e) => eprintln!("Error reading directory: {}", e), // Handle potential errors
        }

        let output_file = format!("{}/chapter_{}.m4a", AUDIO_OUTPUT_DIR, chapter_number);
        combine_chapter(file_paths, &output_file).await;
        match ffmpeg::get_audio_length(&output_file) {
            Ok(length) => chapter_lengths.push(length),
            Err(e) => println!("{}", e),
        }
    }

    let chapter_file = format!("{}/chapter.txt", AUDIO_OUTPUT_DIR);
    let chap_titles: Vec<&str> = titles.iter().map(|s| s.as_str()).collect();

    match ffmpeg::create_chapter_file(chapter_lengths, chap_titles, chapter_file.clone()) {
        Ok(()) => println!("Chapter file created successfully"),
        Err(e) => panic!("Failed to create chapter file: {}", e),
    }

    let audio_path = PathBuf::from(AUDIO_OUTPUT_DIR);
    let mut chapter_files = match get_chap_files(&audio_path) {
        Ok(files) => files,
        Err(e) => panic!("Failed to get chapter files: {}", e),
    };
    chapter_files.sort_by_key(|entry| get_chapter_number(entry).unwrap_or(u32::MAX));

    let output_file = format!("{}/book.m4a", AUDIO_OUTPUT_DIR);
    ffmpeg::add_chapter_data(&chapter_file, chapter_files.to_vec(), &output_file).ok();
    for file in chapter_files {
        println!("Trying to remove file");
        fs::remove_file(file).ok();
    }

    let metadata_map = metdata::get_metadata(opf_file);
    metdata::add_metadata(&output_file, &metadata_map, &cover);
}

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// file
    #[arg(short, long)]
    file: String,

    #[arg(short, long)]
    opf: Option<String>,
    #[arg(short, long)]
    cover: Option<String>,
}

#[tokio::main]
async fn main() {
    fs::create_dir_all(AUDIO_OUTPUT_DIR).ok();
    let args = Args::parse();

    let file_path = args.file;
    let opf_file = args.opf.unwrap_or_else(|| "none.opf".to_string()); // Use a default or handle None case
    let cover = args.cover.unwrap_or_else(|| "none.img".to_string());
    println!("file: {}, opf: {}, cover: {}", file_path, opf_file, cover);

    if file_path.ends_with(".txt") {
        if opf_file != "none.opf" {
            if cover == "none.img" {
                println!("{}", "no cover image provided".yellow())
            }
            make_book(&file_path, &opf_file, &cover).await;
        } else {
            let message = "Missing OPF file";
            println!("{}", message.red())
        }
        // If opf is None, you can provide some default logic for handling
    } else if file_path.ends_with(".epub") {
        println!(
            "{}",
            "Creating Intermediate File You can edit this".yellow()
        );
        epub::make_file(&file_path, "book.txt").ok();
    }
    fs::remove_dir_all(AUDIO_OUTPUT_DIR).ok();
}
