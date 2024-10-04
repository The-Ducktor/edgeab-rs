use image::GenericImageView;
use mp4ameta::{Img, ImgFmt, Tag};
use regex::Regex;
use std::fs::File;
use std::path::Path;
use std::io::BufReader;
use std::process::Command;
use std::{collections::HashMap, fs};
use xmltree::{Element, XMLNode};

// Helper function to extract the text from an element
pub fn get_text_from_element(element: &Element) -> String {
    element
        .children
        .iter()
        .filter_map(|node| {
            if let XMLNode::Text(text) = node {
                Some(text.clone())
            } else {
                None
            }
        })
        .collect::<Vec<_>>()
        .join("")
}
fn square_cover(img_file: &str) {
    // Load the image from file (replace "input.png" with your file path)
    let mut img = image::open(img_file).expect("Failed to open image");

    // Get the dimensions of the original image
    let (width, height) = img.dimensions();

    // Calculate the square side length (based on the width or height, whichever is smaller)
    let square_size = std::cmp::min(width, height);

    // Crop from the top-left corner (0, 0) to the square size
    let cropped_img = img.crop(0, 0, square_size, square_size);

    // Save the cropped image (replace "output.png" with your desired output file path)
    cropped_img
        .save("bcover.png")
        .expect("Failed to save image");
}

fn remove_html_tags(input: &str) -> String {
    // Define a regular expression to match HTML tags
    let re = Regex::new(r"<[^>]*>").unwrap();

    // Replace all HTML tags with an empty string
    let result = re.replace_all(input, "");

    result.to_string()
}

fn _shorten_name(original: &str) -> String {
    original
        .split_whitespace() // Split the string into words
        .filter_map(|word| {
            // Filter out words that are too short (optional)
            if word.len() >= 1 {
                Some(word.chars().next().unwrap())
            } else {
                None
            }
        })
        .collect::<String>() // Collect the characters into a single string
}
fn add_cover_to_m4b(m4b_path: &str, cover_image_path: &str) {
    square_cover(cover_image_path);
    let cover_path = "bcover.png";
    // Read the existing tag from the M4B file
    let mut tag = Tag::read_from_path(m4b_path).expect("Failed to read tag from M4B file");

    // Read the cover image file into a Vec<u8>
    let image_data = fs::read(cover_path).expect("Failed to read cover image file");
    let img_format = ImgFmt::Jpeg; // Set the image format as JPEG

    // Create an Img instance from the image data
    // The Img::new function requires the image data (Vec<u8>) and the format
    let cover = Img::new(img_format, image_data);

    // Set the artwork in the tag
    tag.set_artwork(cover);

    // Write the updated tag back to the M4B file
    tag.write_to_path(m4b_path)
        .expect("Failed to write updated tag to M4B file");
    
    fs::remove_file(Path::new(cover_path)).ok();
}

pub fn add_metadata(
    input_file: &String,
    metadata: Option<&HashMap<String, String>>,
    cover_image: &str,
) {
    let title = metadata
        .and_then(|meta| meta.get("title"))
        .map(|title| remove_html_tags(title))
        .unwrap_or_else(|| "generated_book".to_string());

    let output_file = format!("{}.m4b", title);
    let mut args = vec!["-i", input_file];

    let mut metadata_args = Vec::new();

    if let Some(meta) = metadata {
        for (key, value) in meta {
            let data = format!("{}={}", key, remove_html_tags(value));
            metadata_args.push(data);

            if key == "title" {
                let album_data = format!("album={}", remove_html_tags(value));
                metadata_args.push(album_data);
            }
        }
    }
    if metadata.is_some() {
        for data in &metadata_args {
            args.push("-metadata");
            args.push(&data);
        }
    }

    // Stream mapping: map only audio
    args.push("-map");
    args.push("0:a"); // Only map the audio stream

    // Codec and output file
    args.push("-c");
    args.push("copy");
    args.push(&output_file);

    let output = Command::new("ffmpeg")
        .args(&args)
        .output()
        .expect("Failed to execute FFmpeg");

    if output.status.success() {
        println!("Metadata added successfully to {}", output_file);
        fs::remove_file(input_file).ok(); // Optionally remove the original file
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        eprintln!("FFmpeg error: {}", stderr);
    }
    if cover_image != "none.img" {
        add_cover_to_m4b(&output_file, cover_image);
    } else {
        println!("no cover img provided");
    }
}

pub fn get_metadata(file_path: &str) -> HashMap<String, String> {
    let mut metadata_map = HashMap::new();

    // Open the file and create a buffered reader
    let file = File::open(file_path).expect("Unable to open file");
    let reader = BufReader::new(file);

    // Parse the XML file
    let root: Element = Element::parse(reader).expect("Unable to parse XML");

    // Find relevant elements inside the metadata
    if let Some(metadata_node) = root.get_child("metadata") {
        let title = metadata_node
            .get_child("title")
            .map(|t| get_text_from_element(t))
            .unwrap_or_else(|| "Title not found".to_string());

        let date = metadata_node
            .get_child("date")
            .map(|d| get_text_from_element(d))
            .unwrap_or_else(|| "Date not found".to_string());

        let description = metadata_node
            .get_child("description")
            .map(|desc| get_text_from_element(desc))
            .unwrap_or_else(|| "Description not found".to_string());

        let language = metadata_node
            .get_child("language")
            .map(|lang| get_text_from_element(lang))
            .unwrap_or_else(|| "Language not found".to_string());

        // Collecting all creators (authors)
        let authors: Vec<String> = metadata_node
            .children
            .iter()
            .filter_map(|child| {
                if let XMLNode::Element(elem) = child {
                    if elem.name == "creator" {
                        Some(get_text_from_element(elem))
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
            .collect();

        // Insert metadata into the HashMap
        metadata_map.insert("title".to_string(), title);
        metadata_map.insert("date".to_string(), date);
        metadata_map.insert("description".to_string(), description);
        metadata_map.insert("language".to_string(), language);
        metadata_map.insert("author".to_string(), authors.join(", "));
    }

    metadata_map // Return the HashMap with metadata
}
