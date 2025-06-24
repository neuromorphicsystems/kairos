use crate::protocol;

#[derive(Debug)]
enum FileType {
    Index,
    Raw,
    Samples,
    Metadata,
}

#[derive(Debug)]
struct RecordingFiles {
    index: Option<u64>,
    raw: Option<u64>,
    samples: Option<u64>,
    metadata: Option<u64>,
}

#[derive(Debug)]
enum ConvertedFileType {
    Zip,
}

#[derive(Debug)]
struct ConvertedFiles {
    zip: bool,
}

#[derive(Clone, Copy)]
pub enum Action {
    Rename,
    Delete,
}

pub fn process_write_files(
    directory: &std::path::PathBuf,
    action: Action,
) -> Result<(), std::io::Error> {
    match directory.read_dir() {
        Ok(entries) => {
            for entry in entries {
                let entry = entry?;
                if entry.file_name().to_string_lossy().ends_with(".write") {
                    let path = entry.path();
                    match action {
                        Action::Rename => {
                            let mut new_path = path.clone();
                            new_path.set_extension("");
                            std::fs::rename(path, new_path)?;
                        }
                        Action::Delete => {
                            std::fs::remove_file(path)?;
                        }
                    }
                }
            }
            Ok(())
        }
        Err(_) => Ok(()),
    }
}

//pub fn read_recordings(directory: &std::path::PathBuf) -> Vec<protocol::Recording> {}

fn load_recording_file(
    entry: Result<std::fs::DirEntry, std::io::Error>,
) -> Result<Option<(String, FileType, u64)>, std::io::Error> {
    let entry = entry?;
    if entry.file_type()?.is_file() {
        let path = entry.path();
        match path.file_name() {
            Some(file_name) => {
                let lossy_file_name = file_name.to_string_lossy();
                if lossy_file_name.starts_with(".") {
                    Ok(None)
                } else {
                    let path = if lossy_file_name.ends_with(".write") {
                        let mut new_path = path.clone();
                        new_path.set_extension("");
                        std::fs::rename(path, &new_path)?;
                        new_path
                    } else {
                        path
                    };
                    match path.file_name() {
                        Some(file_name) => {
                            let lossy_file_name = file_name.to_string_lossy();
                            let length = lossy_file_name.chars().count();
                            if lossy_file_name.ends_with(".index.kai") {
                                Ok(Some((
                                    lossy_file_name.chars().take(length - 10).collect(),
                                    FileType::Index,
                                    path.metadata()?.len(),
                                )))
                            } else if lossy_file_name.ends_with(".raw.kai") {
                                Ok(Some((
                                    lossy_file_name.chars().take(length - 8).collect(),
                                    FileType::Raw,
                                    path.metadata()?.len(),
                                )))
                            } else if lossy_file_name.ends_with(".samples.kai") {
                                Ok(Some((
                                    lossy_file_name.chars().take(length - 12).collect(),
                                    FileType::Samples,
                                    path.metadata()?.len(),
                                )))
                            } else if lossy_file_name.ends_with(".toml") {
                                Ok(Some((
                                    lossy_file_name.chars().take(length - 5).collect(),
                                    FileType::Metadata,
                                    path.metadata()?.len(),
                                )))
                            } else {
                                Ok(None)
                            }
                        }
                        None => Ok(None),
                    }
                }
            }
            None => Ok(None),
        }
    } else {
        Ok(None)
    }
}

fn load_converted_file(
    entry: Result<std::fs::DirEntry, std::io::Error>,
) -> Result<Option<(String, ConvertedFileType)>, std::io::Error> {
    let entry = entry?;
    if entry.file_type()?.is_file() {
        let path = entry.path();
        match path.file_name() {
            Some(file_name) => {
                let lossy_file_name = file_name.to_string_lossy();
                if lossy_file_name.starts_with(".") {
                    Ok(None)
                } else if lossy_file_name.ends_with(".write") {
                    std::fs::remove_file(path).map(|_| None)
                } else {
                    let length = lossy_file_name.chars().count();
                    if lossy_file_name.ends_with(".zip") {
                        Ok(Some((
                            lossy_file_name.chars().take(length - 4).collect(),
                            ConvertedFileType::Zip,
                        )))
                    } else {
                        Ok(None)
                    }
                }
            }
            None => Ok(None),
        }
    } else {
        Ok(None)
    }
}

pub fn initialize(
    data_directory: &std::path::PathBuf,
) -> (Vec<protocol::Recording>, Vec<anyhow::Error>) {
    let recordings_directory = data_directory.join("recordings");
    if data_directory.is_dir() && recordings_directory.is_dir() {
        match recordings_directory.read_dir() {
            Ok(recordings_directory_entries) => {
                let mut name_to_recording_files: std::collections::HashMap<String, RecordingFiles> =
                    std::collections::HashMap::new();
                let mut errors = Vec::new();
                for entry in recordings_directory_entries {
                    match load_recording_file(entry) {
                        Ok(Some((name, file_type, size_bytes))) => {
                            if let Some(recording_files) = name_to_recording_files.get_mut(&name) {
                                match file_type {
                                    FileType::Index => {
                                        recording_files.index = Some(size_bytes);
                                    }
                                    FileType::Raw => {
                                        recording_files.raw = Some(size_bytes);
                                    }
                                    FileType::Samples => {
                                        recording_files.samples = Some(size_bytes);
                                    }
                                    FileType::Metadata => {
                                        recording_files.metadata = Some(size_bytes);
                                    }
                                }
                            } else {
                                name_to_recording_files.insert(
                                    name,
                                    RecordingFiles {
                                        index: match file_type {
                                            FileType::Index => Some(size_bytes),
                                            _ => None,
                                        },
                                        raw: match file_type {
                                            FileType::Raw => Some(size_bytes),
                                            _ => None,
                                        },
                                        samples: match file_type {
                                            FileType::Samples => Some(size_bytes),
                                            _ => None,
                                        },
                                        metadata: match file_type {
                                            FileType::Metadata => Some(size_bytes),
                                            _ => None,
                                        },
                                    },
                                );
                            }
                        }
                        Ok(None) => {}
                        Err(error) => errors.push(error.into()),
                    }
                }
                let converted_recordings_directory = data_directory.join("converted-recordings");
                let mut name_to_converted_files: std::collections::HashMap<String, ConvertedFiles> =
                    std::collections::HashMap::new();
                if converted_recordings_directory.is_dir() {
                    match converted_recordings_directory.read_dir() {
                        Ok(converted_recordings_directory_entries) => {
                            for entry in converted_recordings_directory_entries {
                                match load_converted_file(entry) {
                                    Ok(Some((name, file_type))) => {
                                        if let Some(converted_file) =
                                            name_to_converted_files.get_mut(&name)
                                        {
                                            match file_type {
                                                ConvertedFileType::Zip => {
                                                    converted_file.zip = true;
                                                }
                                            }
                                        } else {
                                            name_to_converted_files.insert(
                                                name,
                                                ConvertedFiles {
                                                    zip: matches!(
                                                        file_type,
                                                        ConvertedFileType::Zip
                                                    ),
                                                },
                                            );
                                        }
                                    }
                                    Ok(None) => {}
                                    Err(error) => errors.push(error.into()),
                                }
                            }
                        }
                        Err(error) => {
                            errors.push(error.into());
                        }
                    }
                }
                let mut recordings: Vec<protocol::Recording> = name_to_recording_files
                    .into_iter()
                    .map(|(name, recording_files)| {
                        let mut complete = true;
                        let mut total_size_bytes = 0;
                        match recording_files.index {
                            Some(size_bytes) => {
                                total_size_bytes += size_bytes;
                            }
                            None => {
                                complete = false;
                            }
                        }
                        match recording_files.raw {
                            Some(size_bytes) => {
                                total_size_bytes += size_bytes;
                            }
                            None => {
                                complete = false;
                            }
                        }
                        match recording_files.samples {
                            Some(size_bytes) => {
                                total_size_bytes += size_bytes;
                            }
                            None => {
                                complete = false;
                            }
                        }
                        match recording_files.metadata {
                            Some(size_bytes) => {
                                total_size_bytes += size_bytes;
                            }
                            None => {
                                complete = false;
                            }
                        }
                        let converted_files = name_to_converted_files.get(&name);
                        protocol::Recording {
                            name,
                            state: if complete {
                                protocol::RecordingState::Complete {
                                    size_bytes: total_size_bytes,
                                    zip: converted_files
                                        .map_or(false, |converted_files| converted_files.zip),
                                }
                            } else {
                                protocol::RecordingState::Incomplete {
                                    size_bytes: total_size_bytes,
                                }
                            },
                        }
                    })
                    .collect();
                recordings.sort_by(|a, b| a.name.cmp(&b.name));
                (recordings, errors)
            }
            Err(error) => (Vec::new(), vec![error.into()]),
        }
    } else {
        (Vec::new(), Vec::new())
    }
}
