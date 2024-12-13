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

    // Selectors for chapter titles and content
    let title_selector = Selector::parse("h1, h2, .chapter-title").unwrap();
    let content_selector = Selector::parse("p, .chapter-content").unwrap();

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

                        // Find titles in the document
                        let chapter_titles: Vec<String> = document
                            .select(&title_selector)
                            .map(|title| clean_text(&title.text().collect::<String>()))
                            .filter(|title| !title.is_empty())
                            .collect();

                        // Find content for those titles
                        let chapter_contents: Vec<String> = document
                            .select(&content_selector)
                            .map(|content| clean_text(&content.text().collect::<String>()))
                            .filter(|content| !content.is_empty())
                            .collect();

                        // Process each chapter if titles and contents are non-empty
                        for (title, content) in chapter_titles.iter().zip(chapter_contents.iter()) {
                            // Skip titles with filter phrases
                            if !should_filter(title, &filter_phrases) {
                                let output = format!("# {}\n{}\n\n", title, content);
                                
                                if let Err(e) = output_file.write_all(output.as_bytes()) {
                                    eprintln!("Failed to write to output file: {}", e);
                                    return Err(io::Error::new(
                                        io::ErrorKind::Other,
                                        "Failed to write to output file",
                                    ));
                                }
                            }
                        }
                    }
                }
            }
            Err(e) => {
                eprintln!("Error reading content: {}", e);
            }
        }
    }

    Ok(())
}