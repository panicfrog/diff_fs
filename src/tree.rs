use crate::blob;
use sha1::{Digest, Sha1};
use std::fs;
use std::path::Path;
use anyhow::Result;
use thiserror::Error;

#[derive(Debug)]
enum EntryId {
    /// The SHA1 hash of the file
    Blob(String),
    /// The SHA1 hash of the directory
    Tree(String),
}

impl EntryId {
    /// Compares the type of two `EntryId`s.
    #[inline]
    fn typeOrder(a: &Entry, b: &Entry) -> std::cmp::Ordering {
        match (&a.oid, &b.oid) {
            (EntryId::Blob(_), EntryId::Tree(_)) => std::cmp::Ordering::Less,
            (EntryId::Tree(_), EntryId::Blob(_)) => std::cmp::Ordering::Greater,
            _ => std::cmp::Ordering::Equal,
        }
    }
    fn get_id(&self) -> &str {
        match self {
            EntryId::Blob(sha1) => sha1,
            EntryId::Tree(sha1) => sha1,
        }
    }
}

#[derive(Debug)]
struct Entry {
    /// The name of the file or directory
    name: String,
    /// The SHA1 hash of the file or directory
    oid: EntryId,
}

impl Entry {
    /// get bytes with type(1) + length(2) + oid(20) + name
    fn bytes(&self) -> Result<Vec<u8>> {
        let oid = self.oid.get_id();
        let length = 1 + 2 + oid.len() + self.name.len();
        let mut bytes = Vec::with_capacity(length);
        // 类型1字节
        match &self.oid {
            EntryId::Blob(_) => bytes.push(0),
            EntryId::Tree(_) => bytes.push(1),
        }
        // 长度2字节
        bytes.extend(&(length as u16).to_be_bytes());
        // oid
        bytes.extend(hex_to_bytes(oid)?);
        // 文件名
        bytes.extend(self.name.bytes());
        Ok(bytes)
    }
}

#[derive(Debug)]
struct Tree {
    entries: Vec<Entry>,
}

impl Tree {
    fn sort_entries(&mut self) {
        self.entries.sort_by(|a, b| {
            // 首先按类型排序，文件在前，目录在后
            let type_order = EntryId::typeOrder(a, b);
            if type_order != std::cmp::Ordering::Equal {
                type_order
            } else {
                a.oid.get_id().cmp(&b.oid.get_id())
            }
        });
    }

    pub fn bytes(&self) -> Result<Vec<u8>> {
        let mut bytes = Vec::new();
        let entries_length = self.entries.len();
        // 写入entries的长度
        bytes.extend(&(entries_length as u16).to_be_bytes());
        // 写入entries
        for entry in &self.entries {
            bytes.extend(entry.bytes()?);
        }
        Ok(bytes)
    }

    fn calculate_sha1(&mut self) -> Result<String> {
        self.sort_entries();
        let mut hasher = Sha1::new();
        hasher.update(&self.bytes()?);
        let hash = hasher.finalize();
        Ok(format!("{:x}", hash))
    }
}

fn write_tree<P1, P2>(from: P1, to: P2) -> Result<()> where P1: AsRef<Path>, P2: AsRef<Path> {
    let mut tree = create_tree(from, &mut |t, hash| -> Result<()> {
        let subfolder = &hash[..2];
        let file_name = &hash[2..];
        let subfolder_path = to.as_ref().join(subfolder);
        if !subfolder_path.exists() {
            std::fs::create_dir_all(&subfolder_path)?;
        }
        let dst = subfolder_path.join(file_name);
        if dst.exists() {
            return Ok(());
        }
        std::fs::write(dst, t.bytes()?)?;
        // std::fs::copy(from, dst)?;
        Ok(())
    })?;
    let sha1 = tree.calculate_sha1()?;
    println!("{}", sha1);
    Ok(())
}

/// Creates a `Tree` object from the given path.
fn create_tree<P, F>(path: P, compeleted: &mut F) -> Result<Tree>
where
    P: AsRef<Path>,
    F: FnMut(&Tree, &str) -> Result<()>,
{
    let mut entries = Vec::new();
    for entry in fs::read_dir(path)? {
        let entry = entry.unwrap();
        let name = entry.file_name().into_string().unwrap();
        let path = entry.path();
        if path.is_dir() {
            let mut tree = create_tree(&path, compeleted)?;
            let sha1 = tree.calculate_sha1()?;
            compeleted(&tree, &sha1)?;
            let oid = EntryId::Tree(sha1);
            entries.push(Entry { name, oid });
        } else {
            let oid = EntryId::Blob(blob::calculate_sha1(&path)?);
            entries.push(Entry { name, oid });
        }
    }
    let mut result = Tree { entries };
    let sha1 = result.calculate_sha1()?;
    compeleted(&result, &sha1)?;
    Ok(result)
}

#[derive(Error, Debug)]
pub enum HexError {
    #[error("Invalid hex digit at: {0}")]
    InvalidHexDigit(usize),
}

pub fn hex_to_bytes(hex: &str) -> Result<Vec<u8>> {
    let convert = |c: u8, idx: usize| -> Result<u8> {
        match c {
            b'A'..=b'F' => Ok(c - b'A' + 10),
            b'a'..=b'f' => Ok(c - b'a' + 10),
            b'0'..=b'9' => Ok(c - b'0'),
            _ => return Err(HexError::InvalidHexDigit(idx).into()),
        }
    };
    hex.as_bytes()
        .chunks(2)
        .enumerate()
        .map(|(i, pair)| Ok(convert(pair[0], 2 * i)? << 4 | convert(pair[1], 2 * i + 1)?))
        .collect()
}

#[cfg(test)]
mod tests {
    use sha1::{Digest, Sha1};
    use std::path::PathBuf;
    use super::*;

    #[test]
    fn test_sha1_bytes() {
        // 计算字符串的sha1值, 再转成16进制, 再转回来
       let mut hasher = Sha1::new();
         hasher.update(b"hello world");
        let hash = hasher.finalize();
        println!("{:?}", hash);
        let sha1 = format!("{:x}", hash);
        let r = hex_to_bytes(&sha1).unwrap();
        println!("{:?}", r);
    }

    #[test]
    fn test_create_tree() {
        use super::*;
        use std::fs::File;
        use std::io::Write;

        let dir = PathBuf::from("text_data");
        let subdir1 = dir.join("subdir1");
        let subdir2 = dir.join("subdir2");
        let file1 = dir.join("file1.txt");
        let file2 = subdir1.join("file2.txt");
        let file3 = subdir2.join("file3.txt");

        std::fs::create_dir_all(&subdir1).unwrap();
        std::fs::create_dir_all(&subdir2).unwrap();

        let mut f1 = File::create(&file1).unwrap();
        f1.write_all(b"hello world").unwrap();

        let mut f2 = File::create(&file2).unwrap();
        f2.write_all(b"goodbye world").unwrap();

        let mut f3 = File::create(&file3).unwrap();
        f3.write_all(b"foo bar").unwrap();

        // let mut completed_count = 0;
        let mut completed = |tree: &Tree, sha1: &str| -> Result<()> {
            println!("tree: {:?}, sha1: {}", tree, sha1);
            Ok(())  
        };

        let tree = create_tree(&dir, &mut completed).unwrap();
        assert_eq!(tree.entries.len(), 3);
        std::fs::remove_dir_all(dir).unwrap();
    }
}
