use crate::protocol;

use serde::Serialize;
use std::io::Read;
use std::io::Seek;
use std::io::Write;

#[derive(Clone, Copy)]
enum FileStatus {
    NotFound,
    Write,
    Complete(u64),
}

pub const INDEX_FILE_EXTENSION: &'static str = ".index.kai";
pub const INDEX_FILE_SIGNATURE: &'static str = "KAIROS-INDEX";
pub const RAW_FILE_EXTENSION: &'static str = ".raw.kai";
pub const RAW_FILE_SIGNATURE: &'static str = "KAIROS-RAW";
pub const SAMPLES_FILE_EXTENSION: &'static str = ".samples.kai";
pub const SAMPLES_FILE_SIGNATURE: &'static str = "KAIROS-SAMPLES";
pub const METADATA_FILE_EXTENSION: &'static str = ".toml";
const ZIP_FILE_EXTENSION: &'static str = ".zip";

const RECORDING_FILES_EXTENSIONS: [&'static str; 4] = [
    INDEX_FILE_EXTENSION,
    RAW_FILE_EXTENSION,
    SAMPLES_FILE_EXTENSION,
    METADATA_FILE_EXTENSION,
];

const CONVERTED_FILES_EXTENSIONS: [&'static str; 1] = [ZIP_FILE_EXTENSION];

pub const RECORDINGS_DIRECTORY_NAME: &'static str = "recordings";
pub const CONVERTED_RECORDINGS_DIRECTORY_NAME: &'static str = "converted-recordings";

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

fn read_stem_to_files_statuses<HandleError, const FILE_TYPES: usize>(
    directory: &std::path::PathBuf,
    extensions: &[&str; FILE_TYPES],
    read_size: bool,
    mut handle_error: HandleError,
) -> std::collections::HashMap<String, [FileStatus; FILE_TYPES]>
where
    HandleError: FnMut(anyhow::Error),
{
    if directory.is_dir() {
        match directory.read_dir() {
            Ok(recordings_directory_entries) => {
                let mut stem_to_files_statuses: std::collections::HashMap<
                    String,
                    [FileStatus; FILE_TYPES],
                > = std::collections::HashMap::new();
                for entry in recordings_directory_entries {
                    match entry {
                        Ok(entry) => {
                            let file_name = entry.file_name();
                            let name = file_name.to_string_lossy();
                            if name.starts_with(".") {
                                continue;
                            }
                            let (name_without_write, write) = match name.strip_suffix(".write") {
                                Some(name) => (name, true),
                                None => (name.as_ref(), false),
                            };
                            for (index, extension) in extensions.iter().enumerate() {
                                if let Some(stem) = name_without_write.strip_suffix(extension) {
                                    let file_status = if write {
                                        FileStatus::Write
                                    } else {
                                        if read_size {
                                            match entry.path().metadata() {
                                                Ok(metadata) => {
                                                    FileStatus::Complete(metadata.len())
                                                }
                                                Err(error) => {
                                                    handle_error(error.into());
                                                    break;
                                                }
                                            }
                                        } else {
                                            FileStatus::Complete(0)
                                        }
                                    };
                                    match stem_to_files_statuses.get_mut(stem) {
                                        Some(files) => {
                                            files[index] = file_status;
                                        }
                                        None => {
                                            let mut files = [FileStatus::NotFound; FILE_TYPES];
                                            files[index] = file_status;
                                            stem_to_files_statuses.insert(stem.to_owned(), files);
                                        }
                                    }

                                    break;
                                }
                            }
                        }
                        Err(error) => {
                            handle_error(error.into());
                        }
                    }
                }
                stem_to_files_statuses
            }
            Err(error) => {
                handle_error(error.into());
                std::collections::HashMap::new()
            }
        }
    } else {
        std::collections::HashMap::new()
    }
}

pub fn read_recordings<HandleError>(
    data_directory: &std::path::PathBuf,
    recordings: &mut Vec<protocol::Recording>,
    mut handle_error: HandleError,
) where
    HandleError: FnMut(anyhow::Error),
{
    let stem_to_recording_files_statuses = read_stem_to_files_statuses(
        &data_directory.join(RECORDINGS_DIRECTORY_NAME),
        &RECORDING_FILES_EXTENSIONS,
        true,
        &mut handle_error,
    );
    let stem_to_converted_files_statuses = read_stem_to_files_statuses(
        &data_directory.join(CONVERTED_RECORDINGS_DIRECTORY_NAME),
        &CONVERTED_FILES_EXTENSIONS,
        false,
        &mut handle_error,
    );
    recordings.reserve(stem_to_recording_files_statuses.len());
    for (name, recording_file_statuses) in stem_to_recording_files_statuses.into_iter() {
        let (mut state, _) = recording_file_statuses.iter().fold(
            (
                protocol::RecordingState::Complete {
                    size_bytes: 0,
                    zip: false,
                },
                0,
            ),
            |(state, size_bytes), file_status| {
                use protocol::RecordingState::Complete;
                use protocol::RecordingState::Incomplete;
                use protocol::RecordingState::Ongoing;
                let size_bytes = size_bytes
                    + match file_status {
                        FileStatus::Complete(size_bytes) => *size_bytes,
                        _ => 0,
                    };
                (
                    match file_status {
                        FileStatus::NotFound => Incomplete { size_bytes },
                        FileStatus::Write => match state {
                            Ongoing => Ongoing,
                            Incomplete { .. } => Incomplete { size_bytes },
                            Complete { .. } => Ongoing,
                            _ => unreachable!(),
                        },
                        FileStatus::Complete(_) => match state {
                            Ongoing => Ongoing,
                            Incomplete { .. } => Incomplete { size_bytes },
                            Complete { .. } => Complete {
                                size_bytes,
                                zip: false,
                            },
                            _ => unreachable!(),
                        },
                    },
                    size_bytes,
                )
            },
        );
        match state {
            protocol::RecordingState::Complete { ref mut zip, .. } => {
                if let Some(converted_files_statuses) = stem_to_converted_files_statuses.get(&name)
                {
                    *zip = matches!(converted_files_statuses[0], FileStatus::Complete { .. });
                }
            }
            protocol::RecordingState::Ongoing | protocol::RecordingState::Incomplete {..} => {}
            _ => unreachable!(),
        }
        recordings.push(protocol::Recording { name, state });
    }
    recordings.sort_by(|a, b| a.name.cmp(&b.name));
}

fn zip_options(
    file_metadata: &std::fs::Metadata,
    options: &zip::write::SimpleFileOptions,
) -> zip::write::SimpleFileOptions {
    if let Ok(modified) = file_metadata.modified() {
        let chrono_datetime: chrono::DateTime<chrono::Local> = modified.into();
        let zip_datetime: Result<zip::DateTime, _> = chrono_datetime.naive_local().try_into();
        if let Ok(zip_datetime) = zip_datetime {
            return options.clone().last_modified_time(zip_datetime);
        }
    }
    options.clone()
}

fn read_header(
    file_buffer: &mut Vec<u8>,
    file: &mut std::io::BufReader<std::fs::File>,
    expected_signature: &[u8],
    expected_version: u8,
) -> Result<u8, anyhow::Error> {
    file_buffer.clear();
    file_buffer.resize(expected_signature.len() + 2, 0u8);
    file.read_exact(file_buffer)?;
    if &file_buffer[..expected_signature.len()] != expected_signature {
        return Err(anyhow::anyhow!(
            "bad file signature (expected {:?}, got {:?}",
            expected_signature,
            &file_buffer[..expected_signature.len()],
        ));
    }
    if file_buffer[expected_signature.len()] != expected_version {
        return Err(anyhow::anyhow!(
            "bad version (expected {}, got {}",
            expected_version,
            file_buffer[expected_signature.len()],
        ));
    }
    Ok(file_buffer[expected_signature.len() + 1])
}

pub fn convert(
    data_directory: &std::path::PathBuf,
    name: &str,
    cancelled: std::sync::Arc<std::sync::atomic::AtomicBool>,
) -> Result<(), anyhow::Error> {
    let converted_recordings_directory = data_directory.join(CONVERTED_RECORDINGS_DIRECTORY_NAME);
    std::fs::create_dir_all(&converted_recordings_directory)?;
    let converted_path =
        converted_recordings_directory.join(format!("{}{}", name, ZIP_FILE_EXTENSION));
    if converted_path.is_file() {
        return Ok(());
    }
    let converted_write_path =
        converted_recordings_directory.join(format!("{}{}.write", name, ZIP_FILE_EXTENSION));
    if converted_write_path.is_file() {
        std::fs::remove_file(&converted_write_path)?;
    }
    let recordings_directory = data_directory.join(RECORDINGS_DIRECTORY_NAME);
    {
        let mut zip = zip::ZipWriter::new(std::io::BufWriter::new(std::fs::File::create(
            &converted_write_path,
        )?));
        let options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated)
            .compression_level(Some(6))
            .large_file(true);
        zip.add_directory(name, options.clone())?;
        let mut file_buffer = Vec::new();

        // convert metadata file
        {
            let path = recordings_directory.join(format!("{}{}", name, METADATA_FILE_EXTENSION));
            let file_metadata = path.metadata()?;
            let mut file = std::fs::File::open(recordings_directory.join(&path))?;
            file_buffer.clear();
            file_buffer.reserve(file_metadata.len() as usize);
            file.read_to_end(&mut file_buffer)?;
            let metadata = toml::from_str::<toml::Value>(str::from_utf8(&file_buffer)?)?;
            if cancelled.load(std::sync::atomic::Ordering::Acquire) {
                return Ok(());
            }
            zip.start_file(
                format!("{name}/{name}.json"),
                zip_options(&file_metadata, &options),
            )?;
            let formatter = serde_json::ser::PrettyFormatter::with_indent(b"    ");
            let mut serializer = serde_json::Serializer::with_formatter(&mut zip, formatter);
            metadata.serialize(&mut serializer)?;
            zip.write_all(b"\n")?;
        }
        if cancelled.load(std::sync::atomic::Ordering::Acquire) {
            return Ok(());
        }

        // convert index file and raw events
        {
            let raw_path = recordings_directory.join(format!("{}{}", name, RAW_FILE_EXTENSION));
            let file_metadata = raw_path.metadata()?;
            let mut raw_file = std::io::BufReader::new(std::fs::File::open(&raw_path)?);
            let raw_file_type = read_header(
                &mut file_buffer,
                &mut raw_file,
                RAW_FILE_SIGNATURE.as_bytes(),
                0u8,
            )?;
            let index_path = recordings_directory.join(format!("{}{}", name, INDEX_FILE_EXTENSION));
            let mut index_file = std::io::BufReader::new(std::fs::File::open(&index_path)?);
            let index_file_type = read_header(
                &mut file_buffer,
                &mut index_file,
                INDEX_FILE_SIGNATURE.as_bytes(),
                0u8,
            )?;
            if raw_file_type != index_file_type {
                return Err(anyhow::anyhow!(
                    "the index and raw types are different (raw is {}, index is {})",
                    raw_file_type,
                    index_file_type,
                ));
            }
            match raw_file_type {
                // EVT3
                0 => {
                    file_buffer.clear();
                    file_buffer.resize(4, 0u8);
                    raw_file.read_exact(&mut file_buffer)?;
                    let width = u16::from_le_bytes(file_buffer[0..2].try_into().expect("2 bytes"));
                    let height = u16::from_le_bytes(file_buffer[2..4].try_into().expect("2 bytes"));

                    // polarity events
                    if cancelled.load(std::sync::atomic::Ordering::Acquire) {
                        return Ok(());
                    }
                    zip.start_file(
                        format!("{name}/{name}_events.csv"),
                        zip_options(&file_metadata, &options),
                    )?;
                    let csv_header = format!("t,x@{width},y@{height},on\n");
                    zip.write_all(csv_header.as_bytes())?;
                    let mut index_data = [0u8; 54];
                    let mut raw_file_position = RAW_FILE_SIGNATURE.len() as u64 + 6;
                    let mut previous_adapter: Option<
                        neuromorphic_drivers::adapters::evt3::Adapter,
                    > = None;
                    let mut triggers_bytes = Vec::new();
                    let mut events_bytes = Vec::new();
                    let mut csv_offset = csv_header.len() as u64;
                    let mut csv_offsets = Vec::new();
                    loop {
                        match index_file.read_exact(&mut index_data) {
                            Ok(()) => {
                                let system_time = u64::from_le_bytes(
                                    index_data[0..8].try_into().expect("8 bytes"),
                                );
                                let system_timestamp = u64::from_le_bytes(
                                    index_data[8..16].try_into().expect("8 bytes"),
                                );
                                let _first_after_overflow = index_data[16] == 1;
                                let raw_file_offset = u64::from_le_bytes(
                                    index_data[17..25].try_into().expect("8 bytes"),
                                );
                                let raw_length = u64::from_le_bytes(
                                    index_data[25..33].try_into().expect("8 bytes"),
                                );
                                let state = neuromorphic_drivers::adapters::evt3::State {
                                    t: u64::from_le_bytes(
                                        index_data[33..41].try_into().expect("8 bytes"),
                                    ),
                                    overflows: u32::from_le_bytes(
                                        index_data[41..45].try_into().expect("4 bytes"),
                                    ),
                                    previous_msb_t: u16::from_le_bytes(
                                        index_data[45..47].try_into().expect("2 bytes"),
                                    ),
                                    previous_lsb_t: u16::from_le_bytes(
                                        index_data[47..49].try_into().expect("2 bytes"),
                                    ),
                                    x: u16::from_le_bytes(
                                        index_data[49..51].try_into().expect("2 bytes"),
                                    ),
                                    y: u16::from_le_bytes(
                                        index_data[51..53].try_into().expect("2 bytes"),
                                    ),
                                    polarity: if index_data[53] == 1 {
                                        neuromorphic_drivers::types::Polarity::On
                                    } else {
                                        neuromorphic_drivers::types::Polarity::Off
                                    },
                                };
                                if raw_file_offset != raw_file_position {
                                    return Err(anyhow::anyhow!(
                                        "Position mismatch (the raw file is at position {} but the index points to {})",
                                        raw_file_position,
                                        raw_file_offset
                                    ));
                                }
                                let mut adapter = neuromorphic_drivers::adapters::evt3::Adapter::from_dimensions_and_state(width, height, state);
                                if let Some(previous_adapter) = previous_adapter.as_ref() {
                                    if adapter.state() != previous_adapter.state() {
                                        return Err(anyhow::anyhow!(
                                            "State mismatch (the raw file's state is {:?} but the index contains {:?})",
                                            previous_adapter.state(),
                                            adapter.state()
                                        ));
                                    }
                                }
                                file_buffer.clear();
                                file_buffer.resize(raw_length as usize, 0u8);
                                raw_file.read_exact(&mut file_buffer)?;
                                raw_file_position += raw_length;
                                adapter.convert(
                                    &file_buffer,
                                    |dvs_event| {
                                        let t = dvs_event.t; // unpack field
                                        let x = dvs_event.x; // unpack field
                                        let y = dvs_event.y; // unpack field
                                        let on = dvs_event.polarity as u8; // unpack field
                                        events_bytes
                                            .extend(format!("{t},{x},{y},{on}\n").as_bytes());
                                    },
                                    |trigger_event| {
                                        let t = trigger_event.t; // unpack field
                                        let id = trigger_event.id; // unpack field
                                        let rising = trigger_event.polarity as u8; // unpack field
                                        triggers_bytes.extend(
                                            format!("{system_time},{system_timestamp},{t},{id},{rising}\n")
                                                .as_bytes(),
                                        );
                                    },
                                );
                                zip.write_all(&events_bytes)?;
                                csv_offsets.push(csv_offset);
                                csv_offset += events_bytes.len() as u64;
                                events_bytes.clear();
                                previous_adapter = Some(adapter);
                            }
                            Err(error) if error.kind() == std::io::ErrorKind::UnexpectedEof => {
                                break;
                            }
                            Err(error) => {
                                return Err(error.into());
                            }
                        }
                        if cancelled.load(std::sync::atomic::Ordering::Acquire) {
                            return Ok(());
                        }
                    }

                    // trigger events
                    if cancelled.load(std::sync::atomic::Ordering::Acquire) {
                        return Ok(());
                    }
                    zip.start_file(
                        format!("{name}/{name}_triggers.csv"),
                        zip_options(&file_metadata, &options),
                    )?;
                    zip.write_all(
                        format!("system_time,system_timestamp,t,id,rising\n").as_bytes(),
                    )?;
                    zip.write_all(&triggers_bytes)?;
                    drop(triggers_bytes);

                    // index
                    if cancelled.load(std::sync::atomic::Ordering::Acquire) {
                        return Ok(());
                    }
                    zip.start_file(
                        format!("{name}/{name}_index.csv"),
                        zip_options(&file_metadata, &options),
                    )?;
                    zip.write_all(
                        format!("system_time,system_timestamp,first_after_overflow,t,offset\n")
                            .as_bytes(),
                    )?;
                    index_file.seek(std::io::SeekFrom::Start(
                        INDEX_FILE_SIGNATURE.len() as u64 + 2,
                    ))?;
                    for csv_offset in csv_offsets.into_iter() {
                        index_file.read_exact(&mut index_data)?;
                        let system_time =
                            u64::from_le_bytes(index_data[0..8].try_into().expect("8 bytes"));
                        let system_timestamp =
                            u64::from_le_bytes(index_data[8..16].try_into().expect("8 bytes"));
                        let first_after_overflow = index_data[16];
                        let t = u64::from_le_bytes(index_data[33..41].try_into().expect("8 bytes"));
                        zip.write_all(
                            format!("{system_time},{system_timestamp},{first_after_overflow},{t},{csv_offset}\n")
                                .as_bytes(),
                        )?;
                    }
                }
                _ => {
                    return Err(anyhow::anyhow!(
                        "unsupported raw file type {}",
                        raw_file_type
                    ));
                }
            }
        }
        if cancelled.load(std::sync::atomic::Ordering::Acquire) {
            return Ok(());
        }

        // convert samples file
        {
            let samples_path =
                recordings_directory.join(format!("{}{}", name, SAMPLES_FILE_EXTENSION));
            let file_metadata = samples_path.metadata()?;
            let mut samples_file = std::io::BufReader::new(std::fs::File::open(&samples_path)?);
            let samples_file_type = read_header(
                &mut file_buffer,
                &mut samples_file,
                SAMPLES_FILE_SIGNATURE.as_bytes(),
                0u8,
            )?;
            match samples_file_type {
                // Prophesee EVK4
                0 => {
                    if cancelled.load(std::sync::atomic::Ordering::Acquire) {
                        return Ok(());
                    }
                    zip.start_file(
                        format!("{name}/{name}_samples.csv"),
                        zip_options(&file_metadata, &options),
                    )?;
                    zip.write_all(
                        format!(
                            "system_time,system_timestamp,illuminance_lux,temperature_celsius\n"
                        )
                        .as_bytes(),
                    )?;
                    file_buffer.clear();
                    file_buffer.resize(24, 0u8);
                    loop {
                        match samples_file.read_exact(&mut file_buffer) {
                            Ok(()) => {
                                zip.write_all(
                                    format!(
                                        "{},{},{},{}\n",
                                        u64::from_le_bytes(
                                            file_buffer[0..8].try_into().expect("8 bytes")
                                        ),
                                        u64::from_le_bytes(
                                            file_buffer[8..16].try_into().expect("8 bytes")
                                        ),
                                        f32::from_le_bytes(
                                            file_buffer[16..20].try_into().expect("4 bytes")
                                        ),
                                        f32::from_le_bytes(
                                            file_buffer[20..24].try_into().expect("4 bytes")
                                        ),
                                    )
                                    .as_bytes(),
                                )?;
                            }
                            Err(error) if error.kind() == std::io::ErrorKind::UnexpectedEof => {
                                break;
                            }
                            Err(error) => {
                                return Err(error.into());
                            }
                        }
                    }
                }
                _ => {
                    return Err(anyhow::anyhow!(
                        "unsupported samples file type {}",
                        samples_file_type
                    ));
                }
            }
        }
        if cancelled.load(std::sync::atomic::Ordering::Acquire) {
            return Ok(());
        }
        zip.finish()?;
    }
    std::fs::rename(&converted_write_path, &converted_path)?;
    Ok(())
}
