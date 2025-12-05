use std::{
    fs, io,
    path::{Path, PathBuf},
    sync::mpsc::Sender,
    thread,
};

#[derive(Debug)]
pub enum DeleteError {
    NotNodeModules(PathBuf),
    OutsideRoot(PathBuf),
    NotDirectory(PathBuf),
    Io(PathBuf, io::Error),
}

#[derive(Debug)]
pub enum DeleteEvent {
    Started(String),
    Finished {
        path: String,
        result: Result<(), DeleteError>,
    },
}

impl std::fmt::Display for DeleteError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DeleteError::NotNodeModules(path) => {
                write!(
                    f,
                    "Refused to delete non-node_modules directory: {}",
                    path.display()
                )
            }
            DeleteError::OutsideRoot(path) => {
                write!(
                    f,
                    "Refused to delete outside the scan root: {}",
                    path.display()
                )
            }
            DeleteError::NotDirectory(path) => {
                write!(f, "Path is not a directory: {}", path.display())
            }
            DeleteError::Io(path, err) => write!(f, "Failed to delete {}: {}", path.display(), err),
        }
    }
}

impl std::error::Error for DeleteError {}

pub fn delete_node_modules(target: &Path, root: &Path) -> Result<(), DeleteError> {
    let canonical_target = target
        .canonicalize()
        .map_err(|err| DeleteError::Io(target.to_path_buf(), err))?;

    let canonical_root = root
        .canonicalize()
        .map_err(|err| DeleteError::Io(root.to_path_buf(), err))?;

    if !canonical_target.starts_with(&canonical_root) {
        return Err(DeleteError::OutsideRoot(canonical_target));
    }

    let dir_name = canonical_target
        .file_name()
        .ok_or_else(|| DeleteError::NotNodeModules(canonical_target.clone()))?;

    if dir_name != "node_modules" {
        return Err(DeleteError::NotNodeModules(canonical_target));
    }

    let metadata = fs::metadata(&canonical_target)
        .map_err(|err| DeleteError::Io(canonical_target.clone(), err))?;

    if !metadata.is_dir() {
        return Err(DeleteError::NotDirectory(canonical_target));
    }

    fs::remove_dir_all(&canonical_target).map_err(|err| DeleteError::Io(canonical_target, err))?;

    Ok(())
}

/// Spawn a delete task in a separate thread to avoid blocking the UI loop.
pub fn spawn_delete(target: String, root: PathBuf, tx: Sender<DeleteEvent>) {
    thread::spawn(move || {
        let _ = tx.send(DeleteEvent::Started(target.clone()));
        let result = delete_node_modules(Path::new(&target), &root);
        let _ = tx.send(DeleteEvent::Finished {
            path: target,
            result,
        });
    });
}
