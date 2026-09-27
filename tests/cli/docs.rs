use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use crate::home::{jevpipe, stdout};

fn repository() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}

fn skills() -> Vec<PathBuf> {
    let mut folders: Vec<PathBuf> = fs::read_dir(repository().join("skills"))
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| path.join("SKILL.md").is_file())
        .collect();
    folders.sort();
    assert!(!folders.is_empty(), "no skills/*/SKILL.md found");
    folders
}

fn help(command: &[String]) -> String {
    stdout(jevpipe().args(command).arg("--help"))
}

fn subcommands(help: &str) -> Vec<String> {
    help.lines()
        .skip_while(|line| *line != "Commands:")
        .skip(1)
        .take_while(|line| !line.trim().is_empty())
        .filter_map(|line| line.split_whitespace().next())
        .filter(|name| *name != "help")
        .map(str::to_owned)
        .collect()
}

fn help_flags() -> BTreeSet<String> {
    let mut flags = BTreeSet::new();
    let mut pending = vec![Vec::new()];
    while let Some(command) = pending.pop() {
        let text = help(&command);
        flags.extend(flag_words(&text));
        for name in subcommands(&text) {
            let mut sub = command.clone();
            sub.push(name);
            pending.push(sub);
        }
    }
    flags
}

fn flag_words(text: &str) -> Vec<String> {
    text.match_indices("--")
        .filter(|(at, _)| {
            text[..*at]
                .chars()
                .next_back()
                .is_none_or(|before| !before.is_ascii_alphanumeric() && before != '-')
        })
        .map(|(at, _)| {
            text[at + 2..]
                .chars()
                .take_while(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || *c == '-')
                .collect::<String>()
        })
        .filter(|name| name.starts_with(|c: char| c.is_ascii_lowercase()))
        .map(|name| format!("--{name}"))
        .collect()
}

fn documented_flags(text: &str) -> BTreeSet<String> {
    let mut flags = BTreeSet::new();
    for line in text.lines() {
        for segment in line
            .split('|')
            .filter(|segment| segment.contains("jevpipe"))
        {
            flags.extend(flag_words(segment));
        }
        for span in line.split('`').skip(1).step_by(2) {
            if span.starts_with("--") {
                flags.extend(flag_words(span));
            }
        }
    }
    flags
}

fn assert_flags_exist(document: &Path) {
    let known = help_flags();
    let text = fs::read_to_string(document).unwrap();
    let unknown: Vec<String> = documented_flags(&text)
        .into_iter()
        .filter(|flag| !known.contains(flag))
        .collect();
    assert!(
        unknown.is_empty(),
        "{} uses flags that no jevpipe --help lists: {unknown:?}",
        document.display()
    );
}

fn frontmatter_name(skill: &str) -> Option<&str> {
    skill
        .lines()
        .skip_while(|line| *line != "---")
        .skip(1)
        .take_while(|line| *line != "---")
        .find_map(|line| line.strip_prefix("name:"))
        .map(str::trim)
}

fn link_targets(text: &str) -> Vec<&str> {
    let inline = text
        .split("](")
        .skip(1)
        .filter_map(|rest| rest.split(')').next());
    let references = text
        .lines()
        .map(str::trim_start)
        .filter(|line| line.starts_with('['))
        .filter_map(|line| line.split_once("]:"))
        .map(|(_, target)| target.trim());
    inline.chain(references).collect()
}

#[test]
fn skill_flags_exist_in_the_help() {
    for folder in skills() {
        assert_flags_exist(&folder.join("SKILL.md"));
    }
}

#[test]
fn skill_name_matches_its_folder() {
    for folder in skills() {
        let skill = fs::read_to_string(folder.join("SKILL.md")).unwrap();
        let folder_name = folder.file_name().unwrap().to_str().unwrap();
        assert_eq!(
            frontmatter_name(&skill),
            Some(folder_name),
            "the skill's name must equal its folder name"
        );
    }
}

#[test]
fn readme_flags_exist_in_the_help() {
    assert_flags_exist(&repository().join("README.md"));
}

#[test]
fn readme_has_no_relative_links() {
    let readme = fs::read_to_string(repository().join("README.md")).unwrap();
    let relative: Vec<&str> = link_targets(&readme)
        .into_iter()
        .filter(|target| !target.starts_with("https://"))
        .collect();
    assert!(
        relative.is_empty(),
        "README.md is also the PyPI description, where relative links break: {relative:?}"
    );
}

#[test]
fn link_targets_include_reference_definitions() {
    let text = "[a](https://example.com/a) ![b](b.png)
[c]: docs/c.md
  [d]: #d";
    assert_eq!(
        link_targets(text),
        ["https://example.com/a", "b.png", "docs/c.md", "#d"]
    );
}

#[test]
fn documented_flags_ignore_other_commands() {
    let text = "rg --files -g '*.rs' | jevpipe filter \"q\" --read-files\nuse `--max-cost 0.50` and `git log --oneline`";
    assert_eq!(
        documented_flags(text),
        BTreeSet::from(["--read-files".to_owned(), "--max-cost".to_owned()])
    );
}
