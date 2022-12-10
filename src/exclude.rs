use std::{
    fs::File,
    io::{self, BufRead},
    path::Path,
};

use anyhow::{Context, Result};

/// Load excluded mirror list form file
pub(crate) fn read_exclude_from(file: &Path) -> Result<Vec<String>> {
    let lines =
        io::BufReader::new(File::open(file).with_context(|| {
            format!("Could not open excluded mirror file `{}`", file.display())
        })?)
        .lines();

    let excluded_mirrors: Vec<String> = lines
        .into_iter()
        .filter_map(|line| line.ok())
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .collect();

    Ok(excluded_mirrors)
}

/// Merge all excluded mirror lists, sort and remove duplicates
pub(crate) fn merge_exclude_mirror_list(lists: Vec<Vec<String>>) -> Result<Option<Vec<String>>> {
    let mut list: Vec<String> = lists.into_iter().flatten().collect();
    list.sort();
    list.dedup();

    if list.is_empty() {
        Ok(None)
    } else {
        Ok(Some(list))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exclude_from_file() {
        let excluded_mirrors = read_exclude_from(Path::new(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/",
            "tests/",
            "excluded_mirrors.conf"
        )))
        .unwrap();

        assert_eq!(excluded_mirrors.len(), 2);
        assert_eq!(excluded_mirrors[0], "ban.this.mirror");
        assert_eq!(excluded_mirrors[1], "ban.this-mirror.also");
    }

    #[test]
    fn merge_exclude_mirror() {
        let list1 = vec![
            "block.this.mirror".to_string(),
            "ban.this-mirror.also".to_string(),
            "ban.this.mirror".to_string(),
        ];

        let list2 = read_exclude_from(Path::new(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/",
            "tests/",
            "excluded_mirrors.conf"
        )))
        .unwrap();

        let list = merge_exclude_mirror_list(vec![list1, list2])
            .unwrap()
            .unwrap();
        assert_eq!(list.len(), 3);
        assert_eq!(list[0], "ban.this-mirror.also");
        assert_eq!(list[1], "ban.this.mirror");
        assert_eq!(list[2], "block.this.mirror");
    }
}
