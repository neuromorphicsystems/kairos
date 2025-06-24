use crate::constants;
use anyhow::anyhow;

#[derive(Debug, Clone, serde::Serialize)]
#[serde(tag = "type")]
pub enum Stream {
    Evt3 { width: u16, height: u16 },
    Evk4Samples,
}

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub struct Lookback {
    pub enabled: bool,
    pub maximum_duration_us: u64,
    pub maximum_size_bytes: usize,
}

impl Default for Lookback {
    fn default() -> Self {
        Self {
            enabled: false,
            maximum_duration_us: 10_000_000,
            maximum_size_bytes: 1_024_000_000,
        }
    }
}

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub struct Autostop {
    pub enabled: bool,
    pub duration_us: u64,
}

impl Default for Autostop {
    fn default() -> Self {
        Self {
            enabled: false,
            duration_us: 10_000_000,
        }
    }
}

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub struct Autotrigger {
    pub enabled: bool,
    pub short_sliding_window: usize,
    pub long_sliding_window: usize,
    pub threshold: f32,
}

impl Default for Autotrigger {
    fn default() -> Self {
        Self {
            enabled: false,
            short_sliding_window: 1,
            long_sliding_window: 120,
            threshold: 10.0,
        }
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct Device {
    pub id: u32,
    pub name: String,
    pub serial: String,
    pub speed: String,
    pub bus_number: u8,
    pub address: u8,
    pub streams: Vec<Stream>,
    pub configuration: neuromorphic_drivers::Configuration,
    pub lookback: Lookback,
    pub autostop: Autostop,
    pub autotrigger: Autotrigger,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct SharedClientState {
    pub data_directory: String,
    pub disk_available_and_total_space: Option<(u64, u64)>,
    pub devices: Vec<Device>,
    pub errors: Vec<String>,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(tag = "type")]
pub enum RecordingState {
    Ongoing,
    Incomplete { size_bytes: u64 },
    Complete { size_bytes: u64, zip: bool },
    Queued { size_bytes: u64, zip: bool },
    Converting { size_bytes: u64, zip: bool },
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct Recording {
    pub name: String,
    pub state: RecordingState,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(tag = "type", content = "content")]
pub enum ServerMessage<'a> {
    SharedClientState(&'a SharedClientState),
    Recordings(&'a Vec<Recording>),
}

impl<'a> ServerMessage<'a> {
    pub fn to_bytes(&self) -> Result<Vec<u8>, anyhow::Error> {
        let mut bytes = vec![0u8; 4];
        serde_json::to_writer(&mut bytes, self)?;
        let length = bytes.len();
        bytes[0..4].copy_from_slice(&(length as u32).to_le_bytes());
        if bytes.len() > constants::MESSAGE_MAXIMUM_LENGTH as usize {
            Err(anyhow!(
                "the server message {:?} in serialized form exceeded the maximum length ({} B > {} B)",
                self,
                bytes.len(),
                constants::MESSAGE_MAXIMUM_LENGTH
            ))
        } else {
            Ok(bytes)
        }
    }
}

pub fn stream_description(
    stream_id: u32,
    recommended_buffer_count: u32,
    maximum_length: u32,
) -> [u8; 12] {
    let mut message = [0u8; 12];
    message[0..4].copy_from_slice(&stream_id.to_le_bytes());
    message[4..8].copy_from_slice(&recommended_buffer_count.to_le_bytes());
    message[8..12].copy_from_slice(&maximum_length.to_le_bytes());
    message
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(tag = "type")]
pub enum ClientMessage {
    Ping,
    StartStream {
        stream_id: u32,
    },
    UpdateConfiguration {
        device_id: u32,
        configuration: neuromorphic_drivers::Configuration,
    },
    UpdateLookback {
        device_id: u32,
        lookback: Lookback,
    },
    UpdateAutostop {
        device_id: u32,
        autostop: Autostop,
    },
    UpdateAutotrigger {
        device_id: u32,
        autotrigger: Autotrigger,
    },
    StartRecording {
        device_id: u32,
        name: String,
    },
    StopRecording {
        device_id: u32,
    },
}
