use std::collections::HashMap;
use std::collections::HashSet;
use std::fs;
use std::fs::create_dir_all;
use std::fs::read_to_string;
use std::fs::File;
use std::io;
use std::io::Write;
use std::path::Path;

use serde::Deserialize;
use serde_json::Value;

#[derive(Deserialize)]
struct Ver {
    name: String,
    vers: String,
    features: HashMap<String, Value>,
    features2: Option<HashMap<String, Value>>,
    yanked: bool,
}

fn read_files_in_directory(
    dir_path: &Path,
) -> io::Result<Vec<(String, Vec<(String, Vec<String>)>)>> {
    let mut output = vec![];
    if dir_path.is_dir() {
        for entry in fs::read_dir(dir_path)? {
            let entry = entry?;
            let path = entry.path();

            if let Some(file_name) = path.file_name() {
                if let Some(file_str) = file_name.to_str() {
                    if file_str.starts_with(".") {
                        continue;
                    }
                }
            }

            if path.is_file() {
                let value = format!(
                    "[{}]",
                    read_to_string(&path).unwrap().replace("}\n{", "},{")
                );
                if let Ok(parsed) = serde_json::from_str::<Vec<Ver>>(&value) {
                    let name = parsed.get(0).unwrap().name.clone();
                    let ver: Vec<_> = parsed
                        .into_iter()
                        .filter(|item| !item.yanked)
                        .map(|item| {
                            let mut features =
                                item.features.into_iter().map(|v| v.0).collect::<Vec<_>>();
                            features.append(
                                &mut item
                                    .features2
                                    .unwrap_or_default()
                                    .into_iter()
                                    .map(|v| v.0)
                                    .collect::<Vec<_>>(),
                            );
                            (item.vers, features)
                        })
                        .collect();
                    output.push((name, handle_versions(ver)));
                }
            } else if path.is_dir() {
                output.append(&mut read_files_in_directory(&path)?);
            }
        }
    }
    Ok(output)
}

fn handle_versions(items: Vec<(String, Vec<String>)>) -> Vec<(String, Vec<String>)> {
    let mut output = vec![];
    let mut features: HashSet<String> = HashSet::new();
    for (version, f) in items {
        let mut new_features = vec![];
        for feature in &f {
            if features.get(feature).is_none() {
                new_features.push(format!("+{}", feature));
                features.insert(feature.clone());
            }
        }
        let removed = features
            .iter()
            .filter(|v| !f.contains(*v))
            .cloned()
            .collect::<Vec<_>>();

        for removed in &removed {
            features.remove(removed);
            new_features.push(format!("-{}", removed));
        }
        output.push((version, new_features))
    }
    output
}

fn main() -> io::Result<()> {
    let dir_path = Path::new("./crates-io-index");

    let output = read_files_in_directory(dir_path)?;
    let mut items: HashMap<char, Vec<(String, Vec<(String, Vec<String>)>)>> = HashMap::new();
    for item in output {
        let f = item
            .0
            .chars()
            .next()
            .unwrap()
            .to_lowercase()
            .collect::<String>()
            .chars()
            .next()
            .unwrap();
        items.entry(f).or_insert_with(|| vec![]).push(item);
    }
    create_dir_all("./index").unwrap();
    for (key, mut value) in items {
        value.sort_by(|(a, _), (b, _)| a.cmp(&b));
        let value = value
            .into_iter()
            .map(|v| {
                let str = serde_json::to_string(&v).unwrap();
                str[1..str.len() - 1].to_string()
            })
            .collect::<Vec<_>>()
            .join("\n");
        File::create(format!("./index/{key}.json"))
            .unwrap()
            .write_all(value.as_bytes())
            .unwrap();
    }
    Ok(())
}
