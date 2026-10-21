use super::*;
use std::fs;
use tempfile::TempDir;

#[test]
fn strips_profile_section_from_prompt_without_touching_neighbors() {
    let input = "## Identity\n\nhello\n\n### PROFILE.md\n\nprivate\n\n### Tools\n\nshell\n";
    let output = strip_profile_md_block(input).unwrap();
    assert!(output.contains("## Identity"));
    assert!(output.contains("### Tools"));
    assert!(!output.contains("PROFILE.md"));
    assert!(!output.contains("private"));
}

#[test]
fn leaves_legacy_transcript_and_markdown_source_bytes_unchanged() {
    let workspace = TempDir::new().unwrap();
    let transcript_dir = workspace.path().join("session_raw");
    let companion_dir = workspace.path().join("sessions/2026_05_14");
    fs::create_dir_all(&transcript_dir).unwrap();
    fs::create_dir_all(&companion_dir).unwrap();
    let transcript = transcript_dir.join("old.jsonl");
    let companion = companion_dir.join("old.md");
    let transcript_bytes = br##"{"role":"system","content":"### PROFILE.md\n\nold profile\n\n### Tools\n\nshell"}
"##;
    let companion_bytes = b"## [system]\n\n### PROFILE.md\n\nold profile\n";
    fs::write(&transcript, transcript_bytes).unwrap();
    fs::write(&companion, companion_bytes).unwrap();

    let stats = run(workspace.path()).unwrap();

    assert_eq!(stats.scanned, 0);
    assert_eq!(stats.cleaned, 0);
    assert_eq!(stats.md_companions_altered, 0);
    assert_eq!(fs::read(transcript).unwrap(), transcript_bytes);
    assert_eq!(fs::read(companion).unwrap(), companion_bytes);
}

#[test]
fn preserves_workspace_profile_source_file() {
    let workspace = TempDir::new().unwrap();
    let profile = workspace.path().join("PROFILE.md");
    fs::write(&profile, "profile content").unwrap();

    let stats = run(workspace.path()).unwrap();

    assert!(!stats.profile_md_removed);
    assert_eq!(fs::read_to_string(profile).unwrap(), "profile content");
    assert_eq!(stats.errors, 0);
}
