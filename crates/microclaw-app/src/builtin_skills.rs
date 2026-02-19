use include_dir::{include_dir, Dir, DirEntry};
use std::path::Path;

static BUILTIN_SKILLS_DIR: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/../../skills/built-in");

pub fn ensure_builtin_skills(skills_root: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(skills_root)?;
    copy_missing_entries(&BUILTIN_SKILLS_DIR, skills_root)
}

fn copy_missing_entries(embedded: &Dir<'_>, destination: &Path) -> std::io::Result<()> {
    for entry in embedded.entries() {
        match entry {
            DirEntry::Dir(dir) => {
                let Some(name) = dir.path().file_name() else {
                    continue;
                };
                let next_dest = destination.join(name);
                std::fs::create_dir_all(&next_dest)?;
                copy_missing_entries(dir, &next_dest)?;
            }
            DirEntry::File(file) => {
                let Some(name) = file.path().file_name() else {
                    continue;
                };
                let out_path = destination.join(name);
                if !out_path.exists() {
                    std::fs::write(out_path, file.contents())?;
                }
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_root() -> std::path::PathBuf {
        std::env::temp_dir().join(format!(
            "microclaw_builtin_skills_test_{}",
            uuid::Uuid::new_v4()
        ))
    }

    fn cleanup(path: &Path) {
        let _ = std::fs::remove_dir_all(path);
    }

    #[test]
    fn test_ensure_builtin_skills_writes_missing_files() {
        let root = temp_root();
        let skills_root = root.join("skills");
        ensure_builtin_skills(&skills_root).unwrap();
        let sample = skills_root.join("pdf").join("SKILL.md");
        assert!(sample.exists());
        let content = std::fs::read_to_string(sample).unwrap();
        assert!(!content.trim().is_empty());
        cleanup(&root);
    }

    #[test]
    fn test_ensure_builtin_skills_does_not_overwrite_existing_file() {
        let root = temp_root();
        let skills_root = root.join("skills");
        let custom_pdf = skills_root.join("pdf");
        std::fs::create_dir_all(&custom_pdf).unwrap();
        let custom_file = custom_pdf.join("SKILL.md");
        std::fs::write(&custom_file, "custom-content").unwrap();

        ensure_builtin_skills(&skills_root).unwrap();
        let content = std::fs::read_to_string(custom_file).unwrap();
        assert_eq!(content, "custom-content");
        cleanup(&root);
    }

    #[test]
    fn test_ensure_builtin_skills_includes_new_macos_and_weather_skills() {
        let root = temp_root();
        let skills_root = root.join("skills");
        ensure_builtin_skills(&skills_root).unwrap();

        for skill in [
            "apple-notes",
            "apple-reminders",
            "apple-calendar",
            "weather",
            "find-skills",
        ] {
            let skill_file = skills_root.join(skill).join("SKILL.md");
            assert!(skill_file.exists(), "missing built-in skill: {skill}");
            let content = std::fs::read_to_string(skill_file).unwrap();
            assert!(!content.trim().is_empty(), "empty skill file: {skill}");
        }

        cleanup(&root);
    }
}
