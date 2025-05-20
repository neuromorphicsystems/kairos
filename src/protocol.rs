use crate::constants;
use anyhow::anyhow;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(tag = "type")]
pub enum Stream {
    Evt3 { width: u16, height: u16 },
    Evk4Samples,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Device {
    pub id: u32,
    pub name: String,
    pub serial: String,
    pub speed: String,
    pub bus_number: u8,
    pub address: u8,
    pub streams: Vec<Stream>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SharedClientState {
    pub devices: Vec<Device>,
}

impl SharedClientState {
    pub fn to_bytes(&self) -> Result<Vec<u8>, anyhow::Error> {
        let mut bytes: Vec<u8> = vec![0; 4];
        serde_json::to_writer(&mut bytes, self)?;
        let length = bytes.len();
        bytes[0..4].copy_from_slice(&(length as u32).to_le_bytes());
        if bytes.len() > constants::MESSAGE_MAXIMUM_LENGTH as usize {
            Err(anyhow!(
                "the message {:?} in serialized form exceeded the maximum length ({} B > {} B)",
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

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Lookback {
    maximum_duration_us: u64,
    maximum_size_bytes: usize,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Autotrigger {}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(tag = "type")]
pub enum ClientMessage {
    StartStream {
        id: u32,
    },
    RecordTarget {
        id: u32,
        lookback: Option<Lookback>,
        autotrigger: Option<Autotrigger>,
        path: Option<String>,
    },
}
