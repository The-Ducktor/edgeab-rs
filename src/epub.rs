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
            return Err(io::Error::new(
                io::ErrorKind::Other,
                "Failed to open EPUB file",
            ));
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
            return Err(io::Error::new(
                io::ErrorKind::Other,
                "Failed to create output file",
            ));
        }
    };

    // Define titles to filter out
    let filter_phrases = vec![
        "copyright",
        "landmarks", // check if this will cause issues
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

                            // If we have a previous chapter and it's not filtered, write it to the file
                            if !current_chapter_title.is_empty() && !skip_chapter {
                                // Write full chapter preview to file
                                let output = format!(
                                    "# {}\n{}\n\n",
                                    current_chapter_title, current_chapter_content
                                );
                                if let Err(e) = output_file.write_all(output.as_bytes()) {
                                    eprintln!("Failed to write to output file: {}", e);
                                    return Err(io::Error::new(
                                        io::ErrorKind::Other,
                                        "Failed to write to output file",
                                    ));
                                }
                            }

                            // Check if the new chapter should be skipped
                            skip_chapter = should_filter(&chapter_title, &filter_phrases);

                            // Start a new chapter, clear previous chapter content
                            current_chapter_title = chapter_title;
                            current_chapter_content.clear(); // Clear previous content
                        }

                        // Collect all text content in the current chapter if not skipped
                        if !skip_chapter {
                            let body = document.select(&Selector::parse("body").unwrap()).next();
                            if let Some(body_element) = body {
                                let plain_text = body_element.text().collect::<Vec<_>>().join(" ");
                                let trimmed_text = plain_text.trim();

                                // Append the current text to the chapter content
                                if !current_chapter_title.is_empty()
                                    && !current_chapter_content.contains(&trimmed_text)
                                {
                                    current_chapter_content.push_str(
                                        &trimmed_text
                                            .lines()
                                            .skip(1)
                                            .collect::<Vec<_>>()
                                            .join("\n"),
                                    );
                                    current_chapter_content.push('\n'); // Add a newline for readability
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

    // Write the last chapter if it exists and wasn't filtered
    if !current_chapter_title.is_empty() && !skip_chapter {
        // Write the last chapter to the file
        let output = format!(
            "# {}\n{}\n\n",
            current_chapter_title, current_chapter_content
        );
        if let Err(e) = output_file.write_all(output.as_bytes()) {
            eprintln!("Failed to write to output file: {}", e);
            return Err(io::Error::new(
                io::ErrorKind::Other,
                "Failed to write to output file",
            ));
        }
    }

    Ok(())
}
