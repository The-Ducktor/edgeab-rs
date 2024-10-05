use core::str;
use std::fs::{self, File};
use std::io;
use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};

pub fn is_ffmpeg_installed() -> bool {
    Command::new("ffmpeg")
        .arg("-version")
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

pub fn create_silence_if_not_exists(duration: f64, output_path: &str) {
    if !Path::new(output_path).exists() {
        Command::new("ffmpeg")
            .args(&[
                "-f",
                "lavfi",
                "-i",
                "anullsrc=r=44100:cl=mono",
                "-t",
                &duration.to_string(),
                output_path,
            ])
            .stdout(Stdio::null()) // Hide standard output
            .stderr(Stdio::null())
            .status()
            .expect("Failed to create silence");
    }
}

pub fn create_chapter_file<P: AsRef<Path>>(
    chapter_lengths: Vec<f64>,
    chapter_names: Vec<&str>,
    output_path: P,
) -> io::Result<()> {
    // Create or open the output file
    let mut file = File::create(output_path)?;
    // Write the metadata header
    writeln!(file, ";FFMETADATA1")?;

    // Calculate chapter times and write to the file
    let mut start_time: f64 = 0.0;

    for (i, name) in chapter_names.iter().enumerate() {
        let end_time;
        if i < chapter_lengths.len() {
            end_time = start_time + chapter_lengths[i];
        } else {
            end_time = chapter_lengths.iter().sum();
        }

        writeln!(file, "[CHAPTER]")?;
        writeln!(file, "TIMEBASE=1/1000")?;
        writeln!(file, "START={}", start_time)?;
        writeln!(file, "END={}", end_time)?;
        writeln!(file, "title={}", name)?;

        // Update start_time for the next chapter
        start_time = end_time; // Adding 1 ms to avoid overlap <- That was my downfall aka don't add that 1 ms
    }

    Ok(())
}
pub fn get_audio_length(file_path: &str) -> Result<f64, String> {
    // Prepare the command to call ffprobe
    let output = Command::new("ffprobe")
        .arg("-v")
        .arg("error")
        .arg("-show_entries")
        .arg("format=duration")
        .arg("-of")
        .arg("default=noprint_wrappers=1:nokey=1")
        .arg(file_path)
        .output()
        .map_err(|e| format!("Failed to execute ffprobe: {}", e))?;

    // Check if the command was successful
    if !output.status.success() {
        return Err(format!("ffprobe failed with status: {}", output.status));
    }

    // Convert the output to a string
    let duration_str = str::from_utf8(&output.stdout)
        .map_err(|e| format!("Failed to convert output to string: {}", e))?
        .trim();

    // Parse the duration as f64
    let duration_seconds = duration_str
        .parse::<f64>()
        .map_err(|e| format!("Failed to parse duration: {}", e))?;

    let duration_ms = duration_seconds * 1000.0; // Convert seconds to milliseconds
    Ok(duration_ms) // Return the duration in milliseconds
}

pub fn add_chapter_data(
    chapter_file: &str,
    chapter_files: Vec<String>,
    output_file: &str,
) -> io::Result<()> {
    // Create a temporary file list for ffmpeg to read
    let file_list_path = "file_list.txt";

    // Write the paths of the chapter files to the file_list_path
    {
        let mut file_list = File::create(file_list_path)?;
        for chapter in chapter_files {
            writeln!(file_list, "file '{}'", chapter)?;
            println!("{chapter}");
        }
    }
    let tmp_file = "temp_output_file.m4a";
    // Execute the ffmpeg command
    let _concat_status = Command::new("ffmpeg")
        .arg("-f")
        .arg("concat")
        .arg("-safe")
        .arg("0")
        .arg("-i")
        .arg(file_list_path) // Input file list
        .arg("-c")
        .arg("copy") // Copy the streams
        .arg(tmp_file) // Temporary output file
        .status()?;

    let _chapter_status = Command::new("ffmpeg")
        .arg("-i")
        .arg(tmp_file) // Input file from the first command
        .arg("-i")
        .arg(chapter_file) // Chapter file
        .arg("-map_metadata")
        .arg("1") // Use metadata from the chapter file
        .arg("-c")
        .arg("copy") // Copy the streams
        .arg(output_file) // Final output file
        .status()?;
    fs::remove_file(tmp_file).ok();

    // Optionally, you might want to remove the temporary file list after execution
    fs::remove_file(file_list_path).ok(); // Ignore any error in removing the file
    fs::remove_file(chapter_file).ok();

    Ok(())
}

pub fn concatenate_audio_files(input_files: Vec<String>, output_file: &str) {
    let temp_silence = "silence.wav";
    let silence_duration = 1.0; // Duration of silence in seconds

    // Create silence if it doesn't exist
    create_silence_if_not_exists(silence_duration, temp_silence);

    // Create a temporary file for the concat
    let input_list_file = "inputs.txt";
    let mut file = File::create(input_list_file).expect("Failed to create input list file");

    // Write audio files and silence into the input list
    for i in 0..input_files.len() {
        writeln!(file, "file '{}'", input_files[i]).expect("Failed to write to input list file");
        if i < input_files.len() - 1 {
            writeln!(file, "file '{}'", temp_silence)
                .expect("Failed to write silence to input list file");
        }
    }
    println!("Combining Files With FFmpeg ");
    // Re-encode and concatenate audio files
    let status = Command::new("ffmpeg")
        .args(&[
            "-f",
            "concat",
            "-safe",
            "0",
            "-i",
            input_list_file,
            "-c",
            "aac",
            "-b:a",
            "69k",
            output_file,
        ])
        .stdout(Stdio::null()) // Hide standard output
        .stderr(Stdio::null()) // Hide standard error
        .status()
        .expect("Failed to concatenate audio files");

    if status.success() {
        println!("Audio files concatenated successfully.");
    } else {
        println!("ffmpeg failed with status: {}", status);
    }

    // Cleanup: Remove all input files and temporary files
    fs::remove_file(input_list_file).expect("Failed to remove input list file");
    fs::remove_file(temp_silence).expect("Failed to remove silence file");

    for input_file in input_files {
        fs::remove_file(input_file).expect("Failed to remove input audio file");
    }
}
