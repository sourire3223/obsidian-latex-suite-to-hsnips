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

    // Parse the line as comma-separated key-value pairs
    let parts: Vec<&str> = line.split(',').map(|s| s.trim()).collect();

    for part in parts {
        if part.starts_with("{trigger:") || part.starts_with("trigger:") {
            // Extract trigger
            trigger = part
                .replace("{trigger:", "")
                .replace("trigger:", "")
                .trim()
                .trim_matches('"')
                .to_string();

            // Handle regex triggers
            if trigger.starts_with('/') && trigger.ends_with('/') {
                trigger = trigger[1..trigger.len() - 1].to_string();
            }
        } else if part.starts_with("replacement:") {
            // Extract replacement
            replacement = part
                .replace("replacement:", "")
                .trim()
                .trim_matches('"')
                .to_string();

            // Skip JavaScript function replacements
            if replacement.contains("=>") {
                return String::new();
            }

            // Convert literal \n to actual newlines
            replacement = replacement.replace("\\n", "\n");
        } else if part.starts_with("options:") {
            // Extract options
            options = part
                .replace("options:", "")
                .trim()
                .trim_matches('"')
                .to_string();
        } else if part.starts_with("description:") {
            // Extract description
            description = Some(
                part.replace("description:", "")
                    .trim()
                    .trim_matches('"')
                    .to_string(),
            );
        } else if part.starts_with("priority:") {
            // Extract priority
            priority = Some(
                part.replace("priority:", "")
                    .trim()
                    .trim_matches('"')
                    .to_string(),
            );
        }
    }

    // Generate the output in hsnips format
    if !trigger.is_empty() && !replacement.is_empty() {
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
