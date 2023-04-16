use crate::blob;
use sha1::{Digest, Sha1};
use std::fs;
use std::path::Path;

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
    /// get bytes with  length(2) + type(1) + oid + name
    fn bytes(&self) -> Vec<u8> {
        let oid = self.oid.get_id();
        let length = 2 + 1 + oid.len()  + self.name.len();
        let mut bytes = Vec::with_capacity(length);
        // 长度2字节
        bytes.extend(&(length as u16).to_be_bytes());
        // 类型1字节
        match &self.oid {
            EntryId::Blob(_) => bytes.push(0),
            EntryId::Tree(_) => bytes.push(1),
        }
        // oid
        bytes.extend(oid.bytes());
        // 文件名
        bytes.extend(self.name.bytes());
        bytes
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

    pub fn bytes(&mut self) -> Vec<u8> {
        self.sort_entries();
        let mut bytes = Vec::new();
        let entries_length = self.entries.len();
        // 写入entries的长度
        bytes.extend(&(entries_length as u16).to_be_bytes());
        // 写入entries
        for entry in &self.entries {
            bytes.extend(entry.bytes());
        }
        bytes
    }

    fn calculate_sha1(&mut self) -> String {
        let mut hasher = Sha1::new();
        hasher.update(&self.bytes());
        let hash = hasher.finalize();
        format!("{:x}", hash)
    }
}

/// Creates a `Tree` object from the given path.
fn create_tree<P, F>(path: P, compeleted: &mut F) -> Result<Tree, std::io::Error>
where
    P: AsRef<Path>,
    F: FnMut(&Tree, &str),
{
    let mut entries = Vec::new();
    for entry in fs::read_dir(path)? {
        let entry = entry.unwrap();
        let name = entry.file_name().into_string().unwrap();
        let path = entry.path();
        if path.is_dir() {
            let mut tree = create_tree(&path, compeleted)?;
            let sha1 = tree.calculate_sha1();
            compeleted(&tree, &sha1);
            let oid = EntryId::Tree(sha1);
            entries.push(Entry { name, oid });
        } else {
            let oid = EntryId::Blob(blob::calculate_sha1(&path)?);
            entries.push(Entry { name, oid });
        }
    }
    let mut result = Tree { entries };
    let sha1 = result.calculate_sha1();
    compeleted(&result, &sha1);
    Ok(result)
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;


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
    let mut completed = |tree: &Tree, sha1: &str| {
        println!("tree: {:?}, sha1: {}", tree, sha1);
        // completed_count += 1;
        // match completed_count {
        //     1 => {
        //         assert_eq!(tree.entries.len(), 2);
        //         assert_eq!(tree.entries[0].name, "file1.txt");
        //         assert_eq!(tree.entries[1].name, "subdir1");
        //         assert_eq!(sha1, "d7c8fbbf1e9f3b7c8b4d5c6f5d7d7d7d7d7d7d7d");
        //     }
        //     2 => {
        //         assert_eq!(tree.entries.len(), 1);
        //         assert_eq!(tree.entries[0].name, "file2.txt");
        //         assert_eq!(sha1, "d7c8fbbf1e9f3b7c8b4d5c6f5d7d7d7d7d7d7d7d");
        //     }
        //     3 => {
        //         assert_eq!(tree.entries.len(), 1);
        //         assert_eq!(tree.entries[0].name, "file3.txt");
        //         assert_eq!(sha1, "d7c8fbbf1e9f3b7c8b4d5c6f5d7d7d7d7d7d7d7d");
        //     }
        //     4 => {
        //         assert_eq!(tree.entries.len(), 2);
        //         assert_eq!(tree.entries[0].name, "subdir1");
        //         assert_eq!(tree.entries[1].name, "subdir2");
        //         assert_eq!(sha1, "d7c8fbbf1e9f3b7c8b4d5c6f5d7d7d7d7d7d7d7d");
        //     }
        //     5 => {
        //         assert_eq!(tree.entries.len(), 1);
        //         assert_eq!(tree.entries[0].name, "file2.txt");
        //         assert_eq!(sha1, "d7c8fbbf1e9f3b7c8b4d5c6f5d7d7d7d7d7d7d7d");
        //     }
        //     6 => {
        //         assert_eq!(tree.entries.len(), 1);
        //         assert_eq!(tree.entries[0].name, "file3.txt");
        //         assert_eq!(sha1, "d7c8fbbf1e9f3b7c8b4d5c6f5d7d7d7d7d7d7d7d");
        //     }
        //     7 => {
        //         assert_eq!(tree.entries.len(), 3);
        //         assert_eq!(tree.entries[0].name, "file1.txt");
        //         assert_eq!(tree.entries[1].name, "subdir1");
        //         assert_eq!(tree.entries[2].name, "subdir2");
        //         assert_eq!(sha1, "d7c8fbbf1e9f3b7c8b4d5c6f5d7d7d7d7d7d7d7d");
        //     }
        //     _ => panic!("Unexpected call to completed function"),
        // }
    };

    let mut tree = create_tree(&dir, &mut completed).unwrap();
    assert_eq!(tree.entries.len(), 3);
    // assert_eq!(tree.entries[0].name, "file1.txt");
    // assert_eq!(tree.entries[1].name, "subdir1");
    // assert_eq!(tree.entries[2].name, "subdir2");
    // assert_eq!(tree.calculate_sha1(), "d7c8fbbf1e9f3b7c8b4d5c6f5d7d7d7d7d7d7d7d");
    std::fs::remove_dir_all(dir).unwrap();
}
    
    }