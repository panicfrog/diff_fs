struct Tree {
    entries: Vec<Entry>,
}

enum EntryId {
    /// The SHA1 hash of the file
    Blob(String),
    /// The SHA1 hash of the directory
    Tree(String),
}

struct Entry {
    /// The name of the file or directory
    name: String,
    /// The SHA1 hash of the file or directory
    oid: EntryId
}
