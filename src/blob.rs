use sha1::{Digest, Sha1};
use std::fs::File;
use std::io::{BufReader, Read};
use std::path::Path;

/// Calculates the SHA1 hash of a file located at the given file path.
///
/// # Arguments
///
/// * `file_path` - A `PathBuf` representing the path to the file to be hashed.
///
/// # Returns
///
/// * `Result<String, std::io::Error>` - A `Result` containing the SHA1 hash of the file as a `String` if successful, or an `std::io::Error` if an error occurred while reading the file.
pub fn calculate_sha1<P: AsRef<Path>>(file_path: P) -> Result<String, std::io::Error> {
    let file = File::open(file_path)?;
    let mut reader = BufReader::new(file);
    let mut hasher = Sha1::new();
    let mut buffer = [0; 1024];
    loop {
        let bytes_read = reader.read(&mut buffer)?;
        if bytes_read == 0 {
            break;
        }
        hasher.update(&buffer[..bytes_read]);
    }
    let hash = hasher.finalize();
    Ok(format!("{:x}", hash))
}

/// Copies a file located at the given file path to the specified output directory, if it does not already exist there.
///
/// # Arguments
///
/// * `from` - A `PathBuf` representing the path to the file to be copied.
/// * `to` - A `PathBuf` representing the directory to which the file should be copied.
///
/// # Returns
///
/// * `Result<(), std::io::Error>` - A `Result` containing `()` if the file was successfully copied or already exists in the output directory, or an `std::io::Error` if an error occurred while copying the file or creating the necessary directories.
pub fn write_file_blob<P1: AsRef<Path>, P2: AsRef<Path>>(from: P1, to: P2) -> Result<(), std::io::Error> {
    let hash = calculate_sha1(&from)?;
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
    std::fs::copy(from, dst)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use std::path::PathBuf;

    #[test]
    fn test_calculate_sha1() {
        // Create a temporary file for testing
        let file_path = "test_file1.txt";
        let mut file = File::create(file_path).unwrap();
        file.write_all(b"hello world").unwrap();
        // Calculate the SHA1 hash of the file
        let hash = calculate_sha1(file_path).unwrap();
        // Check that the hash is correct
        assert_eq!(hash, "2aae6c35c94fcfb415dbe95f408b9ce91ee846ed");
        // Delete the temporary file
        std::fs::remove_file(file_path).unwrap();
    }

    #[test]
    fn test_copy_file_to_dir() -> Result<(), std::io::Error> {
        let file_path = "test_file2.txt";
        let mut file = File::create(file_path).unwrap();
        file.write_all(b"hello world").unwrap();
        let output_dir = PathBuf::from("test_output");
        std::fs::create_dir(&output_dir)?;
        write_file_blob(&file_path, &output_dir)?;
        let hash = calculate_sha1(&file_path)?;
        let subfolder = &hash[..2];
        let file_name = &hash[2..];
        let file_path_in_output_dir = output_dir.join(subfolder).join(file_name);
        let mut output_file = File::open(&file_path_in_output_dir)?;
        let mut contents = String::new();
        output_file.read_to_string(&mut contents)?;
        assert_eq!(contents, "hello world");
        // Delete the temporary file and output directory
        std::fs::remove_file(&file_path)?;
        std::fs::remove_dir_all(&output_dir)?;
        Ok(())
    }
}
