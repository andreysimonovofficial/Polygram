use directories::ProjectDirs;
use serde::Deserialize;
use std::{
    collections::HashSet,
    fs,
    path::{Path, PathBuf},
};

const MANIFEST_LIMIT: u64 = 64 * 1024;
const EXTENSION_LIMIT: usize = 100;

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExtensionManifest {
    pub schema_version: u32,
    pub id: String,
    pub name: String,
    pub version: String,
    pub description: String,
    pub blocks: Vec<ExtensionBlock>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct ExtensionBlock {
    pub name: String,
    #[serde(default = "default_block_kind")]
    pub kind: String,
    #[serde(default)]
    pub inputs: Vec<ExtensionMetric>,
    #[serde(default)]
    pub outputs: Vec<ExtensionMetric>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct ExtensionMetric {
    pub name: String,
    #[serde(default)]
    pub value: f64,
    #[serde(default)]
    pub required: f64,
    #[serde(default)]
    pub max: Option<f64>,
    #[serde(default = "default_unit")]
    pub unit: String,
}

#[derive(Clone, Debug)]
pub struct LoadedExtension {
    pub directory: PathBuf,
    pub manifest: ExtensionManifest,
}

#[derive(Clone, Debug)]
pub struct ExtensionFailure {
    pub directory: PathBuf,
    pub reason: String,
}

#[derive(Default)]
pub struct ExtensionScan {
    pub extensions: Vec<LoadedExtension>,
    pub failures: Vec<ExtensionFailure>,
}

fn default_block_kind() -> String {
    "Процесс".to_owned()
}

fn default_unit() -> String {
    "ед.".to_owned()
}

pub fn extension_roots() -> Vec<PathBuf> {
    let mut roots = Vec::new();
    if let Ok(directory) = std::env::current_dir() {
        roots.push(directory.join("extensions"));
    }
    if let Ok(executable) = std::env::current_exe()
        && let Some(directory) = executable.parent()
    {
        roots.push(directory.join("extensions"));
    }
    if let Some(directories) = ProjectDirs::from("com", "Polygram", "Polygram") {
        roots.push(directories.data_local_dir().join("extensions"));
    }
    let mut unique = HashSet::new();
    roots.retain(|root| unique.insert(root.clone()));
    roots
}

pub fn scan_extensions(roots: &[PathBuf]) -> ExtensionScan {
    let mut result = ExtensionScan::default();
    let mut seen_ids = HashSet::new();
    let mut visited = 0;

    for root in roots {
        let entries = match fs::read_dir(root) {
            Ok(entries) => entries,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
            Err(error) => {
                result.failures.push(ExtensionFailure {
                    directory: root.clone(),
                    reason: format!("Не удалось прочитать папку: {error}"),
                });
                continue;
            }
        };

        for entry in entries {
            if visited >= EXTENSION_LIMIT {
                result.failures.push(ExtensionFailure {
                    directory: root.clone(),
                    reason: format!("Превышен лимит расширений: {EXTENSION_LIMIT}"),
                });
                return result;
            }
            let entry = match entry {
                Ok(entry) => entry,
                Err(error) => {
                    result.failures.push(ExtensionFailure {
                        directory: root.clone(),
                        reason: format!("Не удалось прочитать элемент папки: {error}"),
                    });
                    continue;
                }
            };
            if !entry.file_type().is_ok_and(|kind| kind.is_dir()) {
                continue;
            }
            visited += 1;
            let directory = entry.path();
            match load_extension(&directory) {
                Ok(manifest) => {
                    if !seen_ids.insert(manifest.id.clone()) {
                        result.failures.push(ExtensionFailure {
                            directory,
                            reason: format!("Повторяется ID расширения: {}", manifest.id),
                        });
                    } else {
                        result.extensions.push(LoadedExtension {
                            directory,
                            manifest,
                        });
                    }
                }
                Err(reason) => result.failures.push(ExtensionFailure { directory, reason }),
            }
        }
    }
    result
}

fn load_extension(directory: &Path) -> Result<ExtensionManifest, String> {
    let id = directory
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| "Имя папки должно быть корректным ID".to_owned())?;
    if !valid_id(id) {
        return Err("ID: только строчные латинские буквы, цифры и дефисы".to_owned());
    }
    let manifest_path = directory.join("manifest.json");
    let metadata = fs::metadata(&manifest_path)
        .map_err(|error| format!("Не удалось прочитать manifest.json: {error}"))?;
    if !metadata.is_file() || metadata.len() > MANIFEST_LIMIT {
        return Err("manifest.json должен быть файлом размером не более 64 КБ".to_owned());
    }
    let contents = fs::read_to_string(&manifest_path)
        .map_err(|error| format!("Не удалось прочитать manifest.json: {error}"))?;
    let manifest: ExtensionManifest = serde_json::from_str(&contents)
        .map_err(|error| format!("Ошибка manifest.json: {error}"))?;
    if manifest.schema_version != 1 || manifest.id != id {
        return Err("schemaVersion должен быть 1, а id — совпадать с именем папки".to_owned());
    }
    if !valid_text(&manifest.name, 120)
        || !valid_text(&manifest.version, 40)
        || !valid_text(&manifest.description, 500)
        || manifest.blocks.is_empty()
        || manifest.blocks.len() > 100
    {
        return Err("Нужны name, version, description и от 1 до 100 блоков".to_owned());
    }
    for block in &manifest.blocks {
        if !valid_text(&block.name, 120)
            || !valid_text(&block.kind, 80)
            || block.inputs.len() > 100
            || block.outputs.len() > 100
        {
            return Err("У блока некорректное имя, тип или слишком много показателей".to_owned());
        }
        for metric in block.inputs.iter().chain(&block.outputs) {
            if !valid_text(&metric.name, 120)
                || !valid_text(&metric.unit, 30)
                || !metric.value.is_finite()
                || !metric.required.is_finite()
                || metric.required < 0.0
                || metric.max.is_some_and(|max| !max.is_finite() || max < 0.0)
            {
                return Err(
                    "У показателя некорректное имя, единица или числовое значение".to_owned(),
                );
            }
        }
    }
    Ok(manifest)
}

fn valid_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 80
        && id
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
        && !id.starts_with('-')
        && !id.ends_with('-')
        && !id.contains("--")
}

fn valid_text(value: &str, max_length: usize) -> bool {
    !value.trim().is_empty() && value.chars().count() <= max_length
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scans_valid_templates_and_reports_invalid_directories_independently() {
        let root =
            std::env::temp_dir().join(format!("polygram-extensions-{}", uuid::Uuid::new_v4()));
        let valid = root.join("sample-extension");
        let invalid = root.join("invalid-extension");
        fs::create_dir_all(&valid).expect("valid folder is created");
        fs::create_dir_all(&invalid).expect("invalid folder is created");
        fs::write(
            valid.join("manifest.json"),
            r#"{"schemaVersion":1,"id":"sample-extension","name":"Sample","version":"1.0.0","description":"Example template","blocks":[{"name":"Sales","inputs":[{"name":"Leads","required":10}],"outputs":[{"name":"Revenue","unit":"$"}]}]}"#,
        )
        .expect("valid manifest is written");
        fs::write(invalid.join("manifest.json"), "{").expect("invalid manifest is written");

        let result = scan_extensions(std::slice::from_ref(&root));
        assert_eq!(result.extensions.len(), 1);
        assert_eq!(result.extensions[0].manifest.blocks[0].outputs[0].unit, "$");
        assert_eq!(result.failures.len(), 1);

        fs::remove_dir_all(root).expect("temporary extension folder is removed");
    }

    #[test]
    fn bundled_developer_example_is_a_loadable_extension() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("extensions");
        let result = scan_extensions(std::slice::from_ref(&root));
        assert!(
            result
                .extensions
                .iter()
                .any(|extension| extension.manifest.id == "sample-sales"),
            "sample extension should load; failures: {:?}",
            result
                .failures
                .iter()
                .map(|failure| failure.reason.as_str())
                .collect::<Vec<_>>()
        );
    }
}
