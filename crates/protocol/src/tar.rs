use std::process::Command;
use tracing::error;

pub fn tar_workspace(workspace_dir: &str, tar_path: &str) -> std::io::Result<()> {
    let output = Command::new("tar")
        .arg("-cf")
        .arg(tar_path)
        .arg("-C")
        .arg(workspace_dir)
        .arg(".")
        .output()?;

    if !output.status.success() {
        error!(
            "Error creating tarball: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        return Err(std::io::Error::other("Failed to create tarball"));
    }

    Ok(())
}

pub fn untar_workspace(tar_path: &str, dest_dir: &str) -> std::io::Result<()> {
    let output = Command::new("tar")
        .arg("-xf")
        .arg(tar_path)
        .arg("-C")
        .arg(dest_dir)
        .output()?;

    if !output.status.success() {
        error!(
            "Error extracting tarball: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        return Err(std::io::Error::other("Failed to extract tarball"));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tar_workspace() {
        let temp_dir = tempfile::tempdir().unwrap();
        let workspace_dir = temp_dir.path().join("workspace");
        let tar_path = temp_dir.path().join("workspace.tar.gz");

        std::fs::create_dir_all(&workspace_dir).unwrap();

        let test_file = workspace_dir.join("test.txt");
        std::fs::write(&test_file, "test content").unwrap();

        tar_workspace(workspace_dir.to_str().unwrap(), tar_path.to_str().unwrap()).unwrap();

        assert!(tar_path.exists());
    }

    #[test]
    fn test_untar_workspace() {
        let temp_dir = tempfile::tempdir().unwrap();
        let workspace_dir = temp_dir.path().join("workspace");
        let tar_path = temp_dir.path().join("workspace.tar.gz");
        let extract_dir = temp_dir.path().join("extracted");
        std::fs::create_dir_all(&workspace_dir).unwrap();

        let test_file = workspace_dir.join("test.txt");
        std::fs::write(&test_file, "test content").unwrap();

        tar_workspace(workspace_dir.to_str().unwrap(), tar_path.to_str().unwrap()).unwrap();
        std::fs::create_dir_all(&extract_dir).unwrap();
        untar_workspace(tar_path.to_str().unwrap(), extract_dir.to_str().unwrap()).unwrap();

        let extracted_file = extract_dir.join("test.txt");
        assert!(extracted_file.exists());
        let content = std::fs::read_to_string(extracted_file).unwrap();
        assert_eq!(content, "test content");
    }
}
