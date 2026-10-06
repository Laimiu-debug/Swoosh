use crate::{
    identity::digest,
    model::{ContentKind, Entry, Manifest, Selection, ENTRY_LIMIT, TEXT_LIMIT},
};
use anyhow::{bail, Context, Result};
use sha2::{Digest, Sha256};
use std::{
    collections::{HashMap, HashSet},
    fs,
    io::Read,
    path::{Path, PathBuf},
};
use uuid::Uuid;
use walkdir::WalkDir;

#[derive(Clone)]
pub(crate) struct SourcePlan {
    pub selection: Selection,
    pub sources: HashMap<usize, PathBuf>,
    pub text: Option<Vec<u8>>,
}

pub fn validate_relative(path: &str) -> Result<()> {
    if path.is_empty() || path.len() > 1024 || path.starts_with('/') || path.contains('\\') {
        bail!("不安全的文件路径");
    }
    for part in path.split('/') {
        if part.is_empty()
            || part == "."
            || part == ".."
            || part.ends_with(['.', ' '])
            || part
                .chars()
                .any(|c| c.is_control() || "<>:\"|?*".contains(c))
        {
            bail!("文件名包含不支持的字符：{path}");
        }
        let base = part
            .split('.')
            .next()
            .unwrap_or_default()
            .to_ascii_uppercase();
        if matches!(base.as_str(), "CON" | "PRN" | "AUX" | "NUL")
            || (base.len() == 4
                && (base.starts_with("COM") || base.starts_with("LPT"))
                && matches!(base.as_bytes()[3], b'1'..=b'9'))
        {
            bail!("文件名为系统保留名称：{path}");
        }
    }
    Ok(())
}

pub fn validate_manifest(manifest: &Manifest) -> Result<()> {
    if manifest.entries.is_empty()
        || manifest.entries.len() > ENTRY_LIMIT
        || manifest.title.len() > 512
        || manifest.sender.name.len() > 256
        || manifest.nonce.len() != 36
    {
        bail!("传输清单超出限制");
    }
    let mut paths = HashSet::new();
    let mut total = 0u64;
    for entry in &manifest.entries {
        validate_relative(&entry.path)?;
        if !paths.insert(entry.path.to_lowercase()) {
            bail!("重复的文件路径");
        }
        if entry.directory {
            if entry.size != 0 || !entry.sha256.is_empty() {
                bail!("目录清单格式错误");
            }
        } else if entry.sha256.len() != 64 || hex::decode(&entry.sha256).is_err() {
            bail!("文件摘要格式错误");
        }
        total = total.checked_add(entry.size).context("文件大小溢出")?;
    }
    for entry in &manifest.entries {
        let mut parent = Path::new(&entry.path).parent();
        while let Some(path) = parent.filter(|p| !p.as_os_str().is_empty()) {
            let key = path.to_string_lossy().replace('\\', "/").to_lowercase();
            if manifest
                .entries
                .iter()
                .any(|e| e.path.to_lowercase() == key && !e.directory)
            {
                bail!("文件和目录路径冲突");
            }
            parent = path.parent();
        }
    }
    if matches!(manifest.kind, ContentKind::Text)
        && (total > TEXT_LIMIT || manifest.entries.len() != 1 || manifest.entries[0].directory)
    {
        bail!("文字内容超过 1 MiB");
    }
    Ok(())
}

pub(crate) fn collect(paths: Vec<PathBuf>) -> Result<SourcePlan> {
    if paths.is_empty() {
        bail!("请选择文件");
    }
    let mut entries = Vec::new();
    let mut sources = HashMap::new();
    let mut root_names = HashSet::new();
    for root in paths {
        if fs::symlink_metadata(&root)?.file_type().is_symlink() {
            bail!("暂不支持符号链接：{}", root.display());
        }
        let name = root
            .file_name()
            .context("请选择文件或文件夹")?
            .to_str()
            .context("文件名编码不受支持")?
            .to_owned();
        validate_relative(&name)?;
        if !root_names.insert(name.to_lowercase()) {
            bail!("选择的内容有同名文件，请分开发送");
        }
        let parent = root.parent().context("无法读取文件路径")?;
        for item in WalkDir::new(&root).follow_links(false).sort_by_file_name() {
            let item = item.context("读取文件夹失败")?;
            if item.file_type().is_symlink() {
                bail!("文件夹包含符号链接：{}", item.path().display());
            }
            if entries.len() >= ENTRY_LIMIT {
                bail!("单次最多发送 10000 个文件或目录");
            }
            let relative = item
                .path()
                .strip_prefix(parent)?
                .to_str()
                .context("文件名编码不受支持")?
                .replace('\\', "/");
            validate_relative(&relative)?;
            let directory = item.file_type().is_dir();
            if !directory && !item.file_type().is_file() {
                bail!("不支持该文件类型");
            }
            let size = if directory { 0 } else { item.metadata()?.len() };
            let sha256 = if directory {
                String::new()
            } else {
                let mut file = fs::File::open(item.path())?;
                let mut hash = Sha256::new();
                let mut buffer = [0u8; 65536];
                loop {
                    let len = file.read(&mut buffer)?;
                    if len == 0 {
                        break;
                    }
                    hash.update(&buffer[..len]);
                }
                hex::encode(hash.finalize())
            };
            if !directory {
                sources.insert(entries.len(), item.path().to_owned());
            }
            entries.push(Entry {
                path: relative,
                size,
                sha256,
                directory,
            });
        }
    }
    let count = entries.iter().filter(|e| !e.directory).count();
    let title = if root_names.len() == 1 {
        entries[0].path.clone()
    } else {
        format!("{count} 个文件")
    };
    let selection = Selection {
        id: Uuid::new_v4().to_string(),
        title,
        count,
        total_bytes: entries.iter().map(|e| e.size).sum(),
        entries,
    };
    Ok(SourcePlan {
        selection,
        sources,
        text: None,
    })
}

pub(crate) fn text_plan(text: String) -> Result<SourcePlan> {
    if text.trim().is_empty() || text.len() as u64 > TEXT_LIMIT {
        bail!("请输入文字，最多 1 MiB");
    }
    let bytes = text.into_bytes();
    let entry = Entry {
        path: "message.txt".into(),
        size: bytes.len() as u64,
        sha256: digest(&bytes),
        directory: false,
    };
    let selection = Selection {
        id: Uuid::new_v4().to_string(),
        title: "一段文字".into(),
        count: 1,
        total_bytes: entry.size,
        entries: vec![entry],
    };
    Ok(SourcePlan {
        selection,
        sources: HashMap::new(),
        text: Some(bytes),
    })
}
