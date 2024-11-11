use rbook;
use rbook::read::ContentType;
use rbook::Ebook;
use scraper::{Html, Selector};
use std::fs::File;
use std::io::{self, Write};

/// Function to extract chapter previews from an EPUB file and write them to an output file.
/// Filters out chapters with titles containing unwanted phrases.
pub fn make_file(input_epub: &str, output_path: &str) -> io::Result<()> {
    // Creating an epub instance
    let epub = match rbook::Epub::new(input_epub) {
        Ok(epub) => epub,
        Err(e) => {
            eprintln!("Failed to open EPUB file: {}", e);
            return Err(io::Error::new(io::ErrorKind::Other, "Failed to open EPUB file"));
        }
    };

    // Creating a reader instance
    let reader = epub.reader();

    // Selector for chapter elements
    let chapter_selector = Selector::parse("h1, h2[class='chapter']").unwrap();

    // Initialize variables to keep track of chapter content
    let mut current_chapter_title = String::new();
    let mut current_chapter_content = String::new();
    let mut skip_chapter = false; // Flag to skip unwanted chapters and their content

    // Open a file to write chapter previews
    let mut output_file = match File::create(output_path) {
        Ok(file) => file,
        Err(e) => {
            eprintln!("Failed to create output file: {}", e);
            return Err(io::Error::new(io::ErrorKind::Other, "Failed to create output file"));
        }
    };

    // Define titles to filter out
    let filter_phrases = vec![
        "copyright",
        "landmarks",
        "table of contents",
        "illustration",
        "contents",
        "navigation",
    ];

    // Function to check if a title should be filtered out
    fn should_filter(title: &str, filter_phrases: &[&str]) -> bool {
        let lower_title = title.to_lowercase();
        for phrase in filter_phrases {
            if lower_title.contains(phrase) {
                return true;
            }
        }
        false
    }

    // Printing the contents of each page, accumulating text for valid chapters
    for content_result in reader.iter() {
        match content_result {
            Ok(content) => {
                if let Some(media_type) = content.get_content(ContentType::MediaType) {
                    // Ensure we only handle XHTML content
                    if media_type == "application/xhtml+xml" {
                        let html_content = content.to_string();
                        let document = Html::parse_document(&html_content);
                        let titles = document.select(&chapter_selector);

                        for title in titles {
                            let chapter_title = title
                                .text()
                                .collect::<Vec<_>>()
                                .join(" ")
                                .trim()
                                .to_string();

                            // Clean up the chapter title by removing any '#' prefix and trimming spaces
                            let formatted_title = chapter_title
                                .strip_prefix('#')
                                .unwrap_or(&chapter_title)
                                .trim()
                                .replace("\n", " ") // Replace line breaks with spaces in titles
                                .to_string();

                            // If we encounter a new chapter title, write out the previous one
                            if !last_title.is_empty() && !skip_chapter {
                                if !current_chapter_content.trim().is_empty() {
                                    // Write previous chapter content with its title
                                    let output = format!(
                                        "# {}\n{}\n\n",
                                        last_title, current_chapter_content
                                    );
                                    if let Err(e) = output_file.write_all(output.as_bytes()) {
                                        eprintln!("Failed to write to output file: {}", e);
                                        return Err(io::Error::new(io::ErrorKind::Other, "Failed to write to output file"));
                                    }
                                }
                            }

                            // Set the flag for skipping if this chapter should be skipped
                            skip_chapter = should_filter(&formatted_title, &filter_phrases);

                            // Reset content accumulator for the new chapter
                            last_title = formatted_title.clone(); // Use formatted title for the next chapter
                            current_chapter_content.clear(); // Clear previous content
                        }

                        // Now collect content for the chapter (without starting a new chapter yet)
                        if !skip_chapter {
                            let body = document.select(&Selector::parse("body").unwrap()).next();
                            if let Some(body_element) = body {
                                let plain_text = body_element.text().collect::<Vec<_>>().join(" ");

                                // Clean the text: only remove extra whitespace and newlines, but keep formatting
                                let cleaned_text = plain_text
                                    .replace("\r", "")  // Remove carriage returns
                                    .to_string();

                                if !cleaned_text.is_empty() {
                                    // Add accumulated content with appropriate formatting
                                    current_chapter_content.push_str(&cleaned_text);
                                    current_chapter_content.push('\n'); // Add newline for readability
                                }
                            } else {
                                eprintln!("Failed to find body in content.");
                            }
                        }
                    } else {
                        eprintln!("Unexpected media type: {}", media_type);
                    }
                } else {
                    eprintln!("Failed to get media type for content.");
                }
            }
            Err(e) => {
                eprintln!("Error reading content: {}", e);
            }
        }
    }

    // After finishing processing all content, write the last chapter if it exists and wasn't filtered
    if !last_title.is_empty() && !skip_chapter {
        if !current_chapter_content.trim().is_empty() {
            let output = format!(
                "# {}\n{}\n\n",
                last_title, current_chapter_content
            );
            if let Err(e) = output_file.write_all(output.as_bytes()) {
                eprintln!("Failed to write to output file: {}", e);
                return Err(io::Error::new(io::ErrorKind::Other, "Failed to write to output file"));
            }
        }
    }

    Ok(())
}
