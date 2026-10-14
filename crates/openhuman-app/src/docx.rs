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

    let script = r#"
import sys, zipfile, re
try:
    with zipfile.ZipFile(sys.argv[1]) as z:
        xml = z.read('word/document.xml').decode('utf-8')
    text = re.sub(r'<[^>]+>', ' ', xml)
    text = re.sub(r'\s+', ' ', text).strip()
    sys.stdout.write(text)
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
