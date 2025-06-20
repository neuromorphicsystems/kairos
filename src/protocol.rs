use crate::constants;
use anyhow::anyhow;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(tag = "type")]
pub enum Stream {
    Evt3 { width: u16, height: u16 },
    Evk4Samples,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct IntegerParameter {
    pub name: String,
    pub description: String,
    pub value: i32,
    pub minimum: i32,
    pub maximum: i32,
    pub default: i32,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct BooleanParameter {
    pub name: String,
    pub description: String,
    pub value: bool,
    pub default: bool,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(tag = "type")]
pub enum Parameter {
    Integer(IntegerParameter),
    Boolean(BooleanParameter),
}

#[derive(thiserror::Error, Debug, Clone)]
pub enum ParameterError {
    #[error("incompatible parameter type for {name} (expected {expected}, got {got})")]
    IncompatibleTypes {
        name: String,
        expected: String,
        got: String,
    },

    #[error("{name} is out of bound ({value} is not in [{minimum}, {maximum}])")]
    OutOfBounds {
        name: String,
        value: i32,
        minimum: i32,
        maximum: i32,
    },
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(tag = "type")]
pub enum ParameterValue {
    Integer { value: i32 },
    Boolean { value: bool },
}

impl Parameter {
    pub fn u8(name: &str, description: &str, default: u8) -> Self {
        Self::Integer(IntegerParameter {
            name: name.to_owned(),
            description: description.to_owned(),
            value: default as i32,
            minimum: 0,
            maximum: 255,
            default: default as i32,
        })
    }

    pub fn update(&mut self, value: &ParameterValue) -> Result<(), ParameterError> {
        match self {
            Parameter::Integer(parameter) => match value {
                ParameterValue::Integer { value } => {
                    if *value < parameter.minimum || *value > parameter.maximum {
                        Err(ParameterError::OutOfBounds {
                            name: parameter.name.clone(),
                            value: *value,
                            minimum: parameter.minimum,
                            maximum: parameter.maximum,
                        })
                    } else {
                        parameter.value = *value;
                        Ok(())
                    }
                }
                ParameterValue::Boolean { .. } => Err(ParameterError::IncompatibleTypes {
                    name: parameter.name.clone(),
                    expected: "string".to_owned(),
                    got: "boolean".to_owned(),
                }),
            },
            Parameter::Boolean(parameter) => match value {
                ParameterValue::Integer { .. } => Err(ParameterError::IncompatibleTypes {
                    name: parameter.name.clone(),
                    expected: "boolean".to_owned(),
                    got: "string".to_owned(),
                }),
                ParameterValue::Boolean { value } => {
                    parameter.value = *value;
                    Ok(())
                }
            },
        }
    }
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
    pub configuration: neuromorphic_drivers::Configuration,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SharedClientState {
    pub data_directory: String,
    pub disk_available_and_total_space: Option<(u64, u64)>,
    pub devices: Vec<Device>,
    pub errors: Vec<String>,
}

impl SharedClientState {
    pub fn to_bytes(&self) -> Result<Vec<u8>, anyhow::Error> {
        let mut bytes = vec![0u8; 4];
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
pub struct Autotrigger {}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
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
    UpdateAutotrigger {
        enabled: bool,
    },
    UpdateAutostop {
        enabled: bool,
        duration_us: u64,
    },
    UpdateLookback {
        enabled: bool,
        maximum_duration_us: u64,
        maximum_size_bytes: usize,
    },
    StartRecording {
        device_id: u32,
        name: String,
    },
    StopRecording {
        device_id: u32,
    },
}
