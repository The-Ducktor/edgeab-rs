// src/book.rs

use std::fs::File;
use std::io::{self, BufRead};

pub fn read_sections(file_path: &str) -> Vec<Vec<String>> {
    // Open the file in read-only mode
    let file = File::open(file_path).expect("Failed to open file");
    let reader = io::BufReader::new(file);

    // Create a vector to hold the sections
    let mut sections: Vec<Vec<String>> = Vec::new();
    let mut current_section: Vec<String> = Vec::new();

    // Read lines from the file
    for line in reader.lines() {
        let line = line.expect("Failed to read line"); // Unwrap the result
        let line = line.trim().to_string();

        // Check if the line starts with "# "
        if line.starts_with("# ") && line.chars().nth(2) != Some('#') {
            // If we have a current section, push it to sections before starting a new one
            if !current_section.is_empty() {
                sections.push(current_section);
                current_section = Vec::new(); // Start a new section
            }
        }

        // Add the line to the current section if it's not empty
        if line.trim().len() > 0 {
            if line.starts_with("# ") {
                // Check if the third character exists and is not '#'
                if line.chars().nth(2) != Some('#') {
                    current_section.push(line.replace("# ", ""));
                } else {
                    current_section.push(line);
                }
            } else {
                current_section.push(line);
            }
        }
    }

    // Don't forget to add the last section if it exists
    if !current_section.is_empty() {
        sections.push(current_section);
    }

    sections // Return the vector of sections
}

pub fn get_titles(file_path: &str) -> Vec<String> {
    // Open the file in read-only mode
    let file = File::open(file_path).expect("Failed to open file");
    let reader = io::BufReader::new(file);
    let mut chapters = Vec::new();

    // Read lines from the file
    for line in reader.lines() {
        let line = line.expect("Failed to read line"); // Unwrap the result
                                                       // Check if the line starts with "# "
        if line.starts_with("# ") {
            chapters.push(line.replace("# ", "").trim().to_string());
        }
    }
    chapters
}

pub struct Book {
    chapters: Vec<(String, Vec<String>)>,
}

impl Book {
    // Function to create a new, empty book
    pub fn new() -> Self {
        Book {
            chapters: Vec::new(),
        }
    }

    // Method to add a chapter with multiple sections or paragraphs
    pub fn add_chapter(&mut self, title: &str, content: Vec<String>) {
        self.chapters.push((title.to_string(), content));
    }
    // Method to get all chapters
    pub fn get_all_chapters(&self) -> Vec<(&String, &Vec<String>)> {
        self.chapters
            .iter()
            .map(|(title, content)| (title, content))
            .collect()
    }
}
