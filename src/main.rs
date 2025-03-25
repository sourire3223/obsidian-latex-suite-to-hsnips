use std::env;
use std::fs::File;
use std::io::{self, BufRead, BufReader, BufWriter, Write};

fn main() -> io::Result<()> {
    // Parse command-line arguments
    let args: Vec<String> = env::args().collect();
    if args.len() != 4 || args[2] != "-o" {
        eprintln!("Usage: {} <input-file> -o <output-file>", args[0]);
        std::process::exit(1);
    }

    let input_file = &args[1];
    let output_file = &args[3];

    // Read from input file
    let file = File::open(input_file)?;
    let reader = BufReader::new(file);
    let lines: Vec<String> = reader.lines().collect::<Result<_, _>>()?;

    // Write to output file
    let output = File::create(output_file)?;
    let mut writer = BufWriter::new(output);

    let mut active_snippets = 0;
    let mut commented_snippets = 0;

    // Skip the first line if it's just a bracket
    let start_index = if !lines.is_empty() && lines[0].trim() == "[" {
        1
    } else {
        0
    };

    // Skip the last line if it's just a bracket
    let end_index = if !lines.is_empty() && lines.last().unwrap().trim() == "]" {
        lines.len() - 1
    } else {
        lines.len()
    };

    for line in &lines[start_index..end_index] {
        let trimmed = line.trim();

        if trimmed.starts_with("//") {
            // Preserve comments
            writeln!(writer, "# {}", &trimmed[2..].trim())?;
        } else if trimmed.starts_with("{") {
            // Convert snippet definition
            // First, write the original line as a comment
            writeln!(writer, "# {}", line)?;

            // Process and transform the snippet
            let snippet = parse_snippet(trimmed);
            if !snippet.is_empty() {
                writeln!(writer, "{}", snippet)?;
                active_snippets += 1;
            }
        } else if !trimmed.is_empty() {
            // Write non-empty, non-snippet lines as-is
            writeln!(writer, "{}", line)?;
        }
    }

    // Log conversion statistics
    println!("Converted {} snippets to {}", active_snippets, output_file);

    writer.flush()?;
    Ok(())
}

fn parse_snippet(line: &str) -> String {
    // Extract snippet components from the JSON-like format
    let mut trigger = String::new();
    let mut replacement = String::new();
    let mut options = String::new();
    let mut description = None;
    let mut priority = None;

    // Handle various formats of the input line
    let cleaned_line = line
        .trim()
        .trim_matches('\'') // Remove outer single quotes if present
        .trim_start_matches('{')
        .trim_end_matches('}'); // Remove outer braces

    // Split by commas but be careful of commas within quotes
    let mut parts = Vec::new();
    let mut current_part = String::new();
    let mut in_quotes = false;
    let mut escape_next = false;

    for c in cleaned_line.chars() {
        if escape_next {
            current_part.push(c);
            escape_next = false;
            continue;
        }

        match c {
            '\\' => {
                current_part.push('\\');
                escape_next = true;
            }
            '"' => {
                in_quotes = !in_quotes;
                current_part.push(c);
            }
            ',' if !in_quotes => {
                parts.push(current_part.trim().to_string());
                current_part.clear();
            }
            _ => current_part.push(c),
        }
    }

    if !current_part.is_empty() {
        parts.push(current_part.trim().to_string());
    }

    for part in parts {
        let part = part.trim();

        if part.starts_with("trigger:") {
            trigger = extract_quoted_value(part, "trigger:");
        } else if part.starts_with("replacement:") {
            let raw_replacement = extract_quoted_value(part, "replacement:");

            // Skip JavaScript function replacements
            if raw_replacement.contains("=>") {
                return String::new();
            }

            // Handle LaTeX backslashes and newlines
            replacement = raw_replacement.replace("\\n", "\n").replace("\\\\", "\\\\"); // Preserve double backslashes
        } else if part.starts_with("options:") {
            options = extract_quoted_value(part, "options:");
        } else if part.starts_with("description:") {
            description = Some(extract_quoted_value(part, "description:"));
        } else if part.starts_with("priority:") {
            priority = Some(extract_quoted_value(part, "priority:"));
        }
    }

    // Generate the output in hsnips format
    if !trigger.is_empty() {
        // If description is empty or not provided, use the trigger as description
        let desc = match description {
            Some(d) if !d.is_empty() => d,
            _ => trigger.clone(),
        };

        let mapped_options = map_options(&options);

        // Add priority comment if present
        let priority_comment = if let Some(p) = priority {
            format!(" priority: {}", p)
        } else {
            String::new()
        };

        // Format the snippet
        format!(
            "snippet {} \"{}{}\" {}\n{}\nendsnippet",
            trigger, desc, priority_comment, mapped_options, replacement
        )
    } else {
        String::new()
    }
}

// Helper function to extract quoted values properly
fn extract_quoted_value(part: &str, key: &str) -> String {
    let value_part = part.trim_start_matches(key).trim();

    // Handle quoted strings
    if value_part.starts_with('"') && value_part.ends_with('"') {
        value_part[1..value_part.len() - 1].to_string()
    } else {
        // Not quoted or other format
        value_part.to_string()
    }
}

fn map_options(options: &str) -> String {
    // Map LaTeX-Suite options to hsnips options
    let mut mapped = String::new();

    // In OrangeX4's HyperSnips fork, these flags are available:
    // A - Auto expand
    // i - In-word expansion
    // w - Word boundary
    // r - Regex
    // m - Math context
    // t - Text context

    // Add 'A' for automatic expansion
    if options.contains('A') {
        mapped.push('A');
    }

    // Add 'r' for regex mode
    if options.contains('r') {
        mapped.push('r');
    }

    // Add 'w' for word boundary
    if options.contains('w') {
        mapped.push('w');
    }

    // Add 'm' for math mode
    if options.contains('m') {
        mapped.push('m');
    }

    // Add 't' for text mode
    // if options.contains('t') {
    //     mapped.push('t');
    // }

    // In LaTeX-Suite, 'n' means "not in-word"
    // In HyperSnips, 'i' means "in-word" expansion
    // Only add 'i' if 'n' is NOT present (default to in-word expansion)
    if !options.contains('n') {
        mapped.push('i');
    }

    mapped
}
