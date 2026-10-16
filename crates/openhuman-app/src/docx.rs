//! Minimal `.docx` text extraction for artifact previewing.
//!
//! A `.docx` is a ZIP container whose `word/document.xml` holds the text.
//! Rather than pulling the whole `zip` crate into the shell binary, we
//! delegate to the platform `python3` (present on macOS and most Linux
//! systems). If python3 is missing (or the file is not a real docx),
//! extraction fails cleanly and the panel falls back to the file viewer.

use std::path::Path;

/// Extract plain text from a `.docx` file. Returns empty string on failure.
pub fn extract_text(path: &Path) -> Result<String, String> {
    use std::process::Command;

    // Text runs (`<w:t>`) are joined as they are: formatting can split one
    // word across several runs. Paragraphs, line breaks and tabs become
    // whitespace; every other tag is dropped.
    let script = r#"
import sys, zipfile, re, html
try:
    with zipfile.ZipFile(sys.argv[1]) as z:
        xml = z.read('word/document.xml').decode('utf-8')
    xml = re.sub(r'</w:p>', '\n', xml)
    xml = re.sub(r'<w:(br|cr)\b[^>]*/>', '\n', xml)
    xml = re.sub(r'<w:tab\b[^>]*/>', '\t', xml)
    text = html.unescape(re.sub(r'<[^>]+>', '', xml))
    lines = [re.sub(r'[ \t]+', ' ', line).strip() for line in text.split('\n')]
    sys.stdout.write('\n'.join(line for line in lines if line))
except Exception:
    sys.exit(1)
"#;

    let output = Command::new("python3")
        .arg("-c")
        .arg(script)
        .arg(path.as_os_str())
        .output()
        .map_err(|e| format!("python3 unavailable: {e}"))?;

    if !output.status.success() {
        return Err(format!(
            "docx extraction failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}
