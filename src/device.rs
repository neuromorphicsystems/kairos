use crate::constants;
use crate::device;
use crate::protocol;
use crate::recordings;

use neuromorphic_drivers::UsbDevice;
use std::io::Write;

const EVK4_ILLUMINANCE_ALPHA: f64 = 0.000000920554835579854387356562;
const EVK4_ILLUMINANCE_BETA: f64 = -1.009776663165910859376594999048;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DeviceId(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct StreamId(pub u32);

impl StreamId {
    pub fn new(device_id: DeviceId, stream_index: u8) -> Self {
        let mut bytes = [stream_index, 0, 0, 0];
        bytes[1..4].copy_from_slice(&(device_id.0 & 0xFFFFFF).to_le_bytes()[0..3]);
        Self(u32::from_le_bytes(bytes))
    }

    pub fn stream_index(self) -> u8 {
        (self.0 & 0xFF) as u8
    }
}

#[derive(Clone)]
pub enum RecordAction {
    Continue,
    Start(String),
    Stop,
}

#[derive(Clone)]
pub struct RecordConfiguration {
    pub action: RecordAction,
    pub lookback: protocol::Lookback,
    pub autostop: protocol::Autostop,
    pub autotrigger: protocol::Autotrigger,
}

#[derive(Default, Clone)]
struct LookbackState {
    maximum_duration_us: u64,
    duration_us: u64,
    size_bytes: usize,
}

#[derive(Clone)]
struct FileState {
    directory: std::path::PathBuf,
    name: String,
    duration_us: u64,
    size_bytes: u64,
}

#[derive(Debug, Clone, Copy)]
struct AutotriggerState {
    short_value: f32,
    long_value: f32,
    ratio: f32,
    threshold: f32,
}

impl AutotriggerState {
    const fn byte_length() -> usize {
        16
    }

    fn serialize_to(&self, buffer: &mut Vec<u8>) {
        buffer.extend(&self.short_value.to_le_bytes()); // 4
        buffer.extend(&self.long_value.to_le_bytes()); // 4
        buffer.extend(&self.ratio.to_le_bytes()); // 4
        buffer.extend(&self.threshold.to_le_bytes()); // 4
    }
}

fn update_autotrigger_state(current: &mut Option<AutotriggerState>, new: AutotriggerState) {
    match current {
        Some(current) => {
            current.short_value = new.short_value;
            current.long_value = new.long_value;
            current.ratio = current.ratio.max(new.ratio);
            current.threshold = current.threshold.max(new.threshold);
        }
        None => {
            let _ = current.replace(new);
        }
    }
}

struct EventThreadState {
    on_event_rate: f32,
    off_event_rate: f32,
    rising_trigger_count: u32,
    falling_trigger_count: u32,
    lookback_state: Option<LookbackState>,
    file_state: Option<FileState>,
    autotrigger_state: Option<AutotriggerState>,
}

// Updates order
// 1. DeviceProxy (visible to clients via the API) sets `record_configuration` to request record state changes.
// 2. The "event thread" (the loop that calls `next_with_timeout`) periodically checks for changes to record_configuration
// and updates `event_thread_state`.
// 3. The "event thread" creates the metadata file (because the file name is not determined until step 2).
// 4. The "sampler thread" (the loop that calls `illuminance`) periodically checks for changes to `event_thread_state`
// and updates its recording state accordingly.

#[derive(serde::Serialize, Clone)]
pub struct Properties {
    pub name: String,
    pub serial: String,
    pub speed: String,
    pub bus_number: u8,
    pub address: u8,
}
pub struct Device {
    id: DeviceId,
    properties: Properties,
    inner: std::sync::Arc<neuromorphic_drivers::Device>,
    flag:
        neuromorphic_drivers::Flag<neuromorphic_drivers::Error, neuromorphic_drivers::UsbOverflow>,
    record_configuration: std::sync::Arc<std::sync::Mutex<RecordConfiguration>>,
    event_thread_state: std::sync::Arc<std::sync::Mutex<EventThreadState>>,
    running: std::sync::Arc<std::sync::atomic::AtomicBool>,
    configuration_changed: std::sync::Arc<std::sync::atomic::AtomicBool>,
}

pub struct DeviceSampler {
    id: DeviceId,
    inner: std::sync::Arc<neuromorphic_drivers::Device>,
    event_thread_state: std::sync::Arc<std::sync::Mutex<EventThreadState>>,
    running: std::sync::Arc<std::sync::atomic::AtomicBool>,
}

pub struct DeviceProxy {
    pub id: DeviceId,
    pub properties: Properties,
    pub inner: std::sync::Arc<neuromorphic_drivers::Device>,
    pub record_configuration: std::sync::Arc<std::sync::Mutex<RecordConfiguration>>,
    pub configuration_changed: std::sync::Arc<std::sync::atomic::AtomicBool>,
}

pub struct Index<'a> {
    system_time: u64,
    system_timestamp: u64,
    first_after_overflow: bool,
    raw_file_offset: u64,
    raw_length: u64,
    state: &'a neuromorphic_drivers::adapters::State,
}

impl<'a> Index<'a> {
    fn write_to(
        &self,
        index_file: &mut std::io::BufWriter<std::fs::File>,
    ) -> (u64, std::io::Result<()>) {
        match self.state {
            neuromorphic_drivers::adapters::State::Davis346(state) => todo!(),
            neuromorphic_drivers::adapters::State::Dvxplorer(state) => todo!(),
            neuromorphic_drivers::adapters::State::Evt3(state) => {
                let mut data = [0u8; 54];
                data[0..8].copy_from_slice(&self.system_time.to_le_bytes()); // 8
                data[8..16].copy_from_slice(&self.system_timestamp.to_le_bytes()); // 8
                data[16] = self.first_after_overflow as u8; // 1
                data[17..25].copy_from_slice(&self.raw_file_offset.to_le_bytes()); // 8
                data[25..33].copy_from_slice(&self.raw_length.to_le_bytes()); // 8
                data[33..41].copy_from_slice(&state.t.to_le_bytes()); // 8
                data[41..45].copy_from_slice(&state.overflows.to_le_bytes()); // 4
                data[45..47].copy_from_slice(&state.previous_msb_t.to_le_bytes()); // 2
                data[47..49].copy_from_slice(&state.previous_lsb_t.to_le_bytes()); // 2
                data[49..51].copy_from_slice(&state.x.to_le_bytes()); // 2
                data[51..53].copy_from_slice(&state.y.to_le_bytes()); // 2
                data[53] = state.polarity as u8; // 1
                write_all_count(index_file, &data)
            }
        }
    }
}

struct LookbackItem {
    datetime: chrono::DateTime<chrono::Local>,
    system_time: u64,
    system_timestamp: u64,
    first_after_overflow: bool,
    configuration: std::rc::Rc<neuromorphic_drivers::Configuration>,
    state: neuromorphic_drivers::adapters::State,
    end_t: u64,
    raw: Vec<u8>,
}

struct Lookback {
    maximum_packet_size_bytes: usize,
    maximum_duration_us: u64,
    maximum_size_bytes: usize,
    duration_us: u64,
    size_bytes: usize,
    read_index: usize,
    write_index: usize,
    items: Vec<LookbackItem>,
    default_state: neuromorphic_drivers::adapters::State,
    current_configuration: std::rc::Rc<neuromorphic_drivers::Configuration>,
}

fn unwrap_non_empty_lookback(lookback: &Option<Lookback>) -> Option<&Lookback> {
    match lookback {
        Some(lookback) => {
            if lookback.read_index == lookback.write_index {
                None
            } else {
                Some(lookback)
            }
        }
        None => None,
    }
}

impl Lookback {
    fn new(
        maximum_duration_us: u64,
        maximum_size_bytes: usize,
        maximum_packet_size_bytes: usize,
        default_state: neuromorphic_drivers::adapters::State,
        current_configuration: neuromorphic_drivers::Configuration,
    ) -> Self {
        let current_configuration = std::rc::Rc::new(current_configuration);
        let items_length = maximum_size_bytes / maximum_packet_size_bytes;
        let mut items = Vec::with_capacity(items_length);
        for _ in 0..items_length {
            items.push(LookbackItem {
                datetime: chrono::DateTime::default(),
                system_time: 0,
                system_timestamp: 0,
                first_after_overflow: false,
                configuration: current_configuration.clone(),
                state: default_state,
                end_t: 0,
                raw: Vec::new(),
            });
        }
        Self {
            maximum_packet_size_bytes,
            maximum_duration_us,
            maximum_size_bytes,
            duration_us: 0,
            size_bytes: 0,
            read_index: 0,
            write_index: 0,
            items,
            default_state,
            current_configuration,
        }
    }

    fn update_maximum_duration_us(&mut self, new_maximum_duration_us: u64) {
        self.maximum_duration_us = new_maximum_duration_us;
        if self.read_index != self.write_index {
            let end_t =
                self.items[(self.write_index + self.items.len() - 1) % self.items.len()].end_t;
            while (self.read_index + 1) % self.items.len() != self.write_index {
                if end_t - self.items[self.read_index].end_t < self.maximum_duration_us {
                    break;
                }
                self.size_bytes -= self.items[self.read_index].raw.len();
                self.read_index = (self.read_index + 1) % self.items.len();
            }
            self.duration_us =
                self.items[(self.write_index + self.items.len() - 1) % self.items.len()].end_t
                    - self.items[self.read_index].state.current_t();
        }
    }

    fn update_maximum_size_bytes(&mut self, new_maximum_size_bytes: usize) {
        self.maximum_size_bytes = new_maximum_size_bytes;
        let new_items_length = new_maximum_size_bytes / self.maximum_packet_size_bytes;
        if new_items_length != self.items.len() {
            let mut new_items = Vec::with_capacity(new_items_length);
            let count = (new_items_length - 1)
                .min((self.write_index + self.items.len() - self.read_index) % self.items.len());
            let mut index = (self.write_index + self.items.len() - count) % self.items.len();
            self.size_bytes = 0;
            while index != self.write_index {
                let mut new_item = LookbackItem {
                    datetime: self.items[index].datetime,
                    system_time: self.items[index].system_time,
                    system_timestamp: self.items[index].system_timestamp,
                    first_after_overflow: self.items[index].first_after_overflow,
                    state: self.items[index].state,
                    end_t: self.items[index].end_t,
                    raw: Vec::new(),
                    configuration: self.items[index].configuration.clone(),
                };
                std::mem::swap(&mut self.items[index].raw, &mut new_item.raw);
                self.size_bytes += new_item.raw.len();
                new_items.push(new_item);
                index = (index + 1) % self.items.len();
            }
            while new_items.len() < new_items_length {
                new_items.push(LookbackItem {
                    datetime: chrono::DateTime::default(),
                    system_time: 0,
                    system_timestamp: 0,
                    first_after_overflow: false,
                    configuration: self.current_configuration.clone(),
                    state: self.default_state,
                    end_t: 0,
                    raw: Vec::new(),
                });
            }
            std::mem::swap(&mut self.items, &mut new_items);
            self.read_index = 0;
            self.write_index = count;
            // no need to trim for duration because update_maximum_duration_us is
            // always called first (when needed) during a configuration update
            if self.read_index == self.write_index {
                self.duration_us = 0;
            } else {
                self.duration_us =
                    self.items[(self.write_index + self.items.len() - 1) % self.items.len()].end_t
                        - self.items[self.read_index].state.current_t();
            }
        }
    }

    fn push(
        &mut self,
        datetime: chrono::DateTime<chrono::Local>,
        system_time: u64,
        system_timestamp: u64,
        first_after_overflow: bool,
        state: neuromorphic_drivers::adapters::State,
        end_t: u64,
        buffer: &[u8],
        current_configuration: Option<neuromorphic_drivers::Configuration>,
    ) {
        self.items[self.write_index].datetime = datetime;
        self.items[self.write_index].system_time = system_time;
        self.items[self.write_index].system_timestamp = system_timestamp;
        self.items[self.write_index].first_after_overflow = first_after_overflow;
        self.items[self.write_index].state = state;
        self.items[self.write_index].end_t = end_t;
        self.items[self.write_index].raw.resize(buffer.len(), 0);
        self.items[self.write_index].raw.copy_from_slice(buffer);
        if let Some(current_configuration) = current_configuration {
            self.current_configuration = std::rc::Rc::new(current_configuration);
        }
        self.items[self.write_index].configuration = self.current_configuration.clone();
        self.size_bytes += buffer.len();
        self.write_index = (self.write_index + 1) % self.items.len();
        if self.write_index == self.read_index {
            self.size_bytes -= self.items[self.read_index].raw.len();
            self.read_index = (self.read_index + 1) % self.items.len();
        }
        while (self.read_index + 1) % self.items.len() != self.write_index {
            if end_t - self.items[self.read_index].end_t < self.maximum_duration_us {
                break;
            }
            self.size_bytes -= self.items[self.read_index].raw.len();
            self.read_index = (self.read_index + 1) % self.items.len();
        }
        self.duration_us = end_t - self.items[self.read_index].state.current_t();
    }

    fn state(&self) -> LookbackState {
        LookbackState {
            maximum_duration_us: self.maximum_duration_us,
            duration_us: self.duration_us,
            size_bytes: self.size_bytes,
        }
    }
}

struct Davis346UiSample {
    system_time: u64,
    system_timestamp: u64,
    on_event_rate: f32,
    off_event_rate: f32,
    autotrigger_state: AutotriggerState,
}

struct Evk3HdUiSample {
    system_time: u64,
    system_timestamp: u64,
    on_event_rate: f32,
    off_event_rate: f32,
    autotrigger_state: AutotriggerState,
}

struct Evk4UiSample {
    system_time: u64,
    system_timestamp: u64,
    on_event_rate: f32,
    off_event_rate: f32,
    rising_trigger_count: u32,
    falling_trigger_count: u32,
    illuminance: f32,
    temperature: f32,
    autotrigger_state: AutotriggerState,
}

enum UiSample {
    Davis346(Davis346UiSample),
    Evk3Hd(Evk3HdUiSample),
    Evk4(Evk4UiSample),
}

impl UiSample {
    fn byte_length(&self) -> usize {
        match self {
            UiSample::Davis346(_) => todo!(),
            UiSample::Evk3Hd(_) => todo!(),
            UiSample::Evk4(_) => 40 + AutotriggerState::byte_length(),
        }
    }

    fn serialize_to(&self, buffer: &mut Vec<u8>) {
        match self {
            UiSample::Davis346(davis346_ui_sample) => {
                todo!()
            }
            UiSample::Evk3Hd(evk3_hd_ui_sample) => {
                todo!()
            }
            UiSample::Evk4(evk4_ui_sample) => {
                buffer.extend(&evk4_ui_sample.system_time.to_le_bytes()); // 8
                buffer.extend(&evk4_ui_sample.system_timestamp.to_le_bytes()); // 8
                buffer.extend(&evk4_ui_sample.on_event_rate.to_le_bytes()); // 4
                buffer.extend(&evk4_ui_sample.off_event_rate.to_le_bytes()); // 4
                buffer.extend(&evk4_ui_sample.rising_trigger_count.to_le_bytes()); // 4
                buffer.extend(&evk4_ui_sample.falling_trigger_count.to_le_bytes()); // 4
                buffer.extend(&evk4_ui_sample.illuminance.to_le_bytes()); // 4
                buffer.extend(&evk4_ui_sample.temperature.to_le_bytes()); // 4
                evk4_ui_sample.autotrigger_state.serialize_to(buffer); // AutotriggerState::byte_length()
            }
        }
    }
}

#[derive(Clone, Copy)]
struct Davis346FileSample {}

#[derive(Clone, Copy)]
struct Evk3HdFileSample {}

#[derive(Clone, Copy)]
struct Evk4FileSample {
    system_time: u64,
    system_timestamp: u64,
    illuminance: f32,
    temperature: f32,
}

#[derive(Clone, Copy)]
enum FileSample {
    Davis346(Davis346FileSample),
    Evk3Hd(Evk3HdFileSample),
    Evk4(Evk4FileSample),
}

impl From<&UiSample> for FileSample {
    fn from(ui_sample: &UiSample) -> Self {
        match ui_sample {
            UiSample::Davis346(_) => todo!(),
            UiSample::Evk3Hd(_) => todo!(),
            UiSample::Evk4(evk4_ui_sample) => Self::Evk4(Evk4FileSample {
                system_time: evk4_ui_sample.system_time,
                system_timestamp: evk4_ui_sample.system_timestamp,
                illuminance: evk4_ui_sample.illuminance,
                temperature: evk4_ui_sample.temperature,
            }),
        }
    }
}

impl FileSample {
    fn write_to(
        &self,
        samples_file: &mut std::io::BufWriter<std::fs::File>,
    ) -> (u64, std::io::Result<()>) {
        match self {
            FileSample::Davis346(_) => todo!(),
            FileSample::Evk3Hd(_) => todo!(),
            FileSample::Evk4(evk4_file_sample) => {
                let mut data = [0u8; 24];
                data[0..8].copy_from_slice(&evk4_file_sample.system_time.to_le_bytes());
                data[8..16].copy_from_slice(&evk4_file_sample.system_timestamp.to_le_bytes());
                data[16..20].copy_from_slice(&evk4_file_sample.illuminance.to_le_bytes());
                data[20..24].copy_from_slice(&evk4_file_sample.temperature.to_le_bytes());
                write_all_count(samples_file, &data)
            }
        }
    }
}

struct SamplerLookback {
    maximum_duration_us: u64,
    read_index: usize,
    write_index: usize,
    items: Vec<FileSample>,
    default_sample: FileSample,
}

impl SamplerLookback {
    fn new(maximum_duration_us: u64, default_sample: FileSample) -> Self {
        let sampling_period_us = constants::SAMPLING_PERIOD.as_micros() as u64;
        let items_length = 1 + ((maximum_duration_us - 1) / sampling_period_us) as usize; // ceil
        Self {
            maximum_duration_us,
            read_index: 0,
            write_index: 0,
            items: vec![default_sample; items_length],
            default_sample,
        }
    }

    fn update_maximum_duration_us(&mut self, new_maximum_duration_us: u64) {
        let sampling_period_us = constants::SAMPLING_PERIOD.as_micros() as u64;
        self.maximum_duration_us = new_maximum_duration_us;
        let new_items_length = 1 + ((self.maximum_duration_us - 1) / sampling_period_us) as usize; // ceil
        if self.items.len() != new_items_length {
            let mut new_items = Vec::with_capacity(new_items_length);
            let count = (new_items_length - 1)
                .min((self.write_index + self.items.len() - self.read_index) % self.items.len());
            let mut index = (self.write_index + self.items.len() - count) % self.items.len();
            while index != self.write_index {
                new_items.push(self.items[index]);
                index = (index + 1) % self.items.len();
            }
            while new_items.len() < new_items_length {
                new_items.push(self.default_sample);
            }
            std::mem::swap(&mut self.items, &mut new_items);
            self.read_index = 0;
            self.write_index = count;
        }
    }

    fn push(&mut self, sample: FileSample) {
        self.items[self.write_index] = sample;
        self.write_index = (self.write_index + 1) % self.items.len();
        if self.write_index == self.read_index {
            self.read_index = (self.read_index + 1) % self.items.len();
        }
    }
}

fn serialize_record_state_to(
    device_id: DeviceId,
    lookback_state: &Option<LookbackState>,
    name_and_duration_us_and_size_bytes: &Option<(String, u64, u64)>,
    buffer: &mut Vec<u8>,
) {
    buffer.extend_from_slice(&device_id.0.to_le_bytes()); // 4
    match lookback_state {
        Some(lookback_state) => {
            buffer.push(1); // 1
            buffer.extend_from_slice(&lookback_state.duration_us.to_le_bytes()); // 8
            buffer.extend_from_slice(&lookback_state.size_bytes.to_le_bytes());
            // 8
        }
        None => {
            buffer.push(0); // 1
            buffer.extend_from_slice(&(0u64).to_le_bytes()); // 8
            buffer.extend_from_slice(&(0u64).to_le_bytes()); // 8
        }
    }
    match name_and_duration_us_and_size_bytes {
        Some(name_and_duration_us_and_size_bytes) => {
            buffer.push(1); // 1
            buffer.extend_from_slice(&name_and_duration_us_and_size_bytes.1.to_le_bytes()); // 8
            buffer.extend_from_slice(&name_and_duration_us_and_size_bytes.2.to_le_bytes()); // 8
            buffer.extend_from_slice(name_and_duration_us_and_size_bytes.0.as_bytes());
        }
        None => {
            buffer.push(0); // 1
            buffer.extend_from_slice(&(0u64).to_le_bytes()); // 8
            buffer.extend_from_slice(&(0u64).to_le_bytes()); // 8
        }
    }
}

struct Recording {
    directory: std::path::PathBuf,
    name: String,
    raw_file: Option<std::io::BufWriter<std::fs::File>>,
    raw_file_error: bool,
    raw_file_offset: u64,
    index_file: Option<std::io::BufWriter<std::fs::File>>,
    index_file_error: bool,
    metadata_file: Option<std::io::BufWriter<std::fs::File>>,
    metadata_file_error: bool,
    start_t: u64,
    size_bytes: u64,
}

struct SamplerRecording {
    directory: std::path::PathBuf,
    name: String,
    samples_file: Option<std::io::BufWriter<std::fs::File>>,
    samples_file_error: bool,
    size_bytes: u64,
}

fn write_all_count(
    file: &mut std::io::BufWriter<std::fs::File>,
    mut buffer: &[u8],
) -> (u64, std::io::Result<()>) {
    let mut written = 0;
    while !buffer.is_empty() {
        match file.write(buffer) {
            Ok(0) => {
                return (
                    written,
                    Err(std::io::Error::new(
                        std::io::ErrorKind::WriteZero,
                        "failed to write whole buffer",
                    )),
                );
            }
            Ok(length) => {
                written += length as u64;
                buffer = &buffer[length..]
            }
            Err(ref error) if error.kind() == std::io::ErrorKind::Interrupted => {}
            Err(error) => return (written, Err(error)),
        }
    }
    (written, Ok(()))
}

fn index_file_path(directory: &std::path::PathBuf, name: &str, write: bool) -> std::path::PathBuf {
    directory.join(format!(
        "{}{}{}",
        name,
        recordings::INDEX_FILE_EXTENSION,
        if write { ".write" } else { "" }
    ))
}

fn raw_file_path(directory: &std::path::PathBuf, name: &str, write: bool) -> std::path::PathBuf {
    directory.join(format!(
        "{}{}{}",
        name,
        recordings::RAW_FILE_EXTENSION,
        if write { ".write" } else { "" }
    ))
}

fn metadata_file_path(
    directory: &std::path::PathBuf,
    name: &str,
    write: bool,
) -> std::path::PathBuf {
    directory.join(format!(
        "{}{}{}",
        name,
        recordings::METADATA_FILE_EXTENSION,
        if write { ".write" } else { "" }
    ))
}

fn samples_file_path(
    directory: &std::path::PathBuf,
    name: &str,
    write: bool,
) -> std::path::PathBuf {
    directory.join(format!(
        "{}{}{}",
        name,
        recordings::SAMPLES_FILE_EXTENSION,
        if write { ".write" } else { "" }
    ))
}

#[derive(serde::Serialize)]
struct Autotrigger {
    timestamp: String,
    short_sliding_window: usize,
    long_sliding_window: usize,
    threshold: f32,
}

enum Trigger {
    Manual(String),
    Auto(Autotrigger),
}

macro_rules! register {
    ($($module:ident),+) => {
        paste::paste! {
            $(
                #[derive(serde::Serialize)]
                struct [<$module:camel Configuration>]<'a> {
                    configuration: &'a neuromorphic_drivers::$module::Configuration,
                }
            )+

            $(
                #[derive(serde::Serialize)]
                struct [<$module:camel ConfigurationUpdate>]<'a> {
                    timestamp: String,
                    configuration: &'a neuromorphic_drivers::$module::Configuration,
                }
            )+

            $(
                #[derive(serde::Serialize)]
                struct [<$module:camel ConfigurationUpdates>]<'a> {
                    configuration_updates: [&'a [<$module:camel ConfigurationUpdate>]<'a>; 1],
                }
            )+

            fn configuration_to_toml(configuration: &neuromorphic_drivers::Configuration) -> String {
                match configuration {
                    $(
                        neuromorphic_drivers::Configuration::[<$module:camel>](configuration) => {
                            toml::to_string(&[<$module:camel Configuration>] {
                                configuration,
                            }).expect("TOML serialization failed")
                        }
                    )+
                }
            }

            fn configuration_update_to_toml(
                datetime: &chrono::DateTime<chrono::Local>,
                configuration: &neuromorphic_drivers::Configuration
            ) -> String {
                match configuration {
                    $(
                        neuromorphic_drivers::Configuration::[<$module:camel>](configuration) => {
                            format!(
                                "\n\n{}",
                                toml::to_string(&[<$module:camel ConfigurationUpdates>] {
                                    configuration_updates: [&[<$module:camel ConfigurationUpdate>] {
                                        timestamp: crate::utc_string(datetime),
                                        configuration,
                                    }],
                                }).expect("TOML serialization failed")
                            )
                        }
                    )+
                }
            }
        }
    }
}

register! { inivation_davis346, inivation_dvxplorer, prophesee_evk3_hd, prophesee_evk4 }

impl Recording {
    // An adapter's internal variables are split into "constants" and "variables" (state)
    // The constants are either immutable but needed for decoding (like width and height),
    // or too big to be appended to buffers (like frames in a DAVIS).
    // The current adapter can be passed to new (pseudo-constants like frames are not persisted),
    // but the state must match the first buffer (the distinction matters for lookback data).
    fn new(
        directory: std::path::PathBuf,
        name: String,
        datetime: &chrono::DateTime<chrono::Local>,
        trigger: Trigger,
        adapter: &neuromorphic_drivers::adapters::Adapter,
        state: &neuromorphic_drivers::adapters::State,
        properties: &Properties,
        configuration: &neuromorphic_drivers::Configuration,
    ) -> Result<Recording, std::io::Error> {
        let mut size_bytes = 0;
        let mut raw_file = std::io::BufWriter::new(std::fs::File::create(raw_file_path(
            &directory, &name, true,
        ))?);
        // 0 is the file version number (not a string terminator)
        raw_file.write_all(format!("{}\0", recordings::RAW_FILE_SIGNATURE).as_bytes())?;
        match adapter {
            neuromorphic_drivers::adapters::Adapter::Davis346(adapter) => todo!(),
            neuromorphic_drivers::adapters::Adapter::Dvxplorer(adapter) => todo!(),
            neuromorphic_drivers::adapters::Adapter::Evt3(adapter) => {
                // format id (0 is EVT3)
                raw_file.write_all(b"\0")?;
                raw_file.write_all(&(adapter.width().to_le_bytes()))?; // 2
                raw_file.write_all(&(adapter.height().to_le_bytes()))?; // 2
            }
        }
        let raw_file_offset = recordings::RAW_FILE_SIGNATURE.len() as u64 + 6;
        size_bytes += raw_file_offset;
        let mut index_file = std::io::BufWriter::new(std::fs::File::create(index_file_path(
            &directory, &name, true,
        ))?);
        // 0 is the file version number (not a string terminator)
        index_file.write_all(format!("{}\0", recordings::INDEX_FILE_SIGNATURE).as_bytes())?;
        match adapter {
            neuromorphic_drivers::adapters::Adapter::Davis346(_) => todo!(),
            neuromorphic_drivers::adapters::Adapter::Dvxplorer(_) => todo!(),
            neuromorphic_drivers::adapters::Adapter::Evt3(_) => {
                // format id (0 is EVT3)
                index_file.write_all(b"\0")?;
            }
        }
        size_bytes += 14;
        let mut metadata_file = std::io::BufWriter::new(std::fs::File::create(
            metadata_file_path(&directory, &name, true),
        )?);
        {
            let datetime_string = format!("timestamp = \"{}\"\n\n", crate::utc_string(datetime));
            metadata_file.write_all(datetime_string.as_bytes())?;
            size_bytes += datetime_string.len() as u64;
        }
        {
            let trigger_string = format!(
                "[trigger]\nmode = \"{}\"\n{}\n",
                match trigger {
                    Trigger::Manual(_) => "manual",
                    Trigger::Auto(_) => "auto",
                },
                match trigger {
                    Trigger::Manual(datetime) => format!("timestamp = \"{datetime}\"\n"),
                    Trigger::Auto(autotrigger) =>
                        toml::to_string(&autotrigger).expect("TOML serialization failed"),
                },
            );
            metadata_file.write_all(trigger_string.as_bytes())?;
            size_bytes += trigger_string.len() as u64;
        }
        {
            let properties_string = format!(
                "[device]\n{}\n",
                toml::to_string(&properties).expect("TOML serialization failed")
            );
            metadata_file.write_all(properties_string.as_bytes())?;
            size_bytes += properties_string.len() as u64;
        }
        {
            let configuration_string = configuration_to_toml(configuration);
            metadata_file.write_all(configuration_string.as_bytes())?;
            size_bytes += configuration_string.len() as u64;
        }
        Ok(Recording {
            directory,
            name,
            raw_file: Some(raw_file),
            raw_file_error: false,
            raw_file_offset,
            index_file: Some(index_file),
            index_file_error: false,
            metadata_file: Some(metadata_file),
            metadata_file_error: false,
            start_t: state.current_t(),
            size_bytes,
        })
    }

    fn update_file_state(&self, current_t: u64, file_state: &mut Option<FileState>) {
        match file_state {
            Some(file_state) => {
                if file_state.directory != self.directory {
                    file_state.directory = self.directory.clone();
                }
                if file_state.name != self.name {
                    file_state.name = self.name.clone();
                }
                file_state.duration_us = current_t.max(self.start_t) - self.start_t;
                file_state.size_bytes = self.size_bytes;
            }
            None => {
                let _ = file_state.replace(FileState {
                    directory: self.directory.clone(),
                    name: self.name.clone(),
                    duration_us: current_t.max(self.start_t) - self.start_t,
                    size_bytes: self.size_bytes,
                });
            }
        }
    }
}

impl Drop for Recording {
    fn drop(&mut self) {
        let _ = self.raw_file.take();
        let _ = std::fs::rename(
            raw_file_path(&self.directory, &self.name, true),
            raw_file_path(&self.directory, &self.name, false),
        );
        let _ = self.index_file.take();
        let _ = std::fs::rename(
            index_file_path(&self.directory, &self.name, true),
            index_file_path(&self.directory, &self.name, false),
        );

        let _ = self.metadata_file.take();
        let _ = std::fs::rename(
            metadata_file_path(&self.directory, &self.name, true),
            metadata_file_path(&self.directory, &self.name, false),
        );
    }
}

fn create_new_recording(
    lookback: &Option<Lookback>,
    now: &chrono::DateTime<chrono::Local>,
    name: &str,
    adapter: &neuromorphic_drivers::Adapter,
    properties: &Properties,
    new_configuration: &Option<neuromorphic_drivers::Configuration>,
    device: &neuromorphic_drivers::Device,
    event_thread_state: &std::sync::Arc<std::sync::Mutex<EventThreadState>>,
    context: &std::sync::Arc<tokio::sync::Mutex<crate::Context>>,
    autostop_reference_t: &mut u64,
    trigger: Trigger,
) -> Option<Recording> {
    let directory = {
        let mut context_guard = context.blocking_lock();
        let directory = std::path::PathBuf::from(&context_guard.shared_client_state.data_directory)
            .join("recordings");
        if let Err(error) = std::fs::create_dir_all(&directory) {
            context_guard.shared_client_state.errors.push(format!(
                "Creating \"{}\" failed ({})",
                directory.to_string_lossy(),
                error
            ));
            return None;
        }
        directory
    };
    let datetime = match unwrap_non_empty_lookback(lookback) {
        Some(lookback) => &lookback.items[lookback.read_index].datetime,
        None => now,
    };
    let timestamp = crate::utc_string_path_safe(datetime);
    let name = if name.is_empty() {
        timestamp
    } else {
        format!("{timestamp}_{name}")
    };
    let new_recording = match unwrap_non_empty_lookback(&lookback) {
        Some(lookback) => Recording::new(
            directory.clone(),
            name.clone(),
            datetime,
            trigger,
            adapter,
            &lookback.items[lookback.read_index].state,
            properties,
            &lookback.items[lookback.read_index].configuration,
        ),
        None => Recording::new(
            directory.clone(),
            name.clone(),
            datetime,
            trigger,
            &adapter,
            &adapter.state(),
            properties,
            &match new_configuration.as_ref() {
                Some(new_configuration) => new_configuration.clone(),
                None => device.current_configuration(),
            },
        ),
    };
    match new_recording {
        Ok(mut new_recording) => {
            {
                let mut event_thread_state_guard = event_thread_state
                    .lock()
                    .expect("event thread state mutex is poisoned");
                new_recording.update_file_state(
                    adapter.current_t(),
                    &mut event_thread_state_guard.file_state,
                );
            }
            if let Some(lookback) = unwrap_non_empty_lookback(&lookback) {
                if let Some(raw_file) = new_recording.raw_file.as_mut() {
                    if let Some(index_file) = new_recording.index_file.as_mut() {
                        if let Some(metadata_file) = new_recording.metadata_file.as_mut() {
                            let mut previous_configuration =
                                lookback.items[lookback.read_index].configuration.clone();
                            let mut index = lookback.read_index;
                            while index != lookback.write_index {
                                let raw_file_offset = new_recording.raw_file_offset;
                                let (count, result) =
                                    write_all_count(raw_file, &lookback.items[index].raw);
                                new_recording.raw_file_offset += count;
                                new_recording.size_bytes += count;
                                if let Err(error) = result {
                                    if !new_recording.raw_file_error {
                                        new_recording.raw_file_error = true;
                                        context.blocking_lock().shared_client_state.errors.push(
                                            format!(
                                                "Writing to \"{}\" failed ({})",
                                                raw_file_path(
                                                    &new_recording.directory,
                                                    &new_recording.name,
                                                    true
                                                )
                                                .to_string_lossy(),
                                                error
                                            ),
                                        );
                                    }
                                }
                                let (count, result) = Index {
                                    system_time: lookback.items[index].system_time,
                                    system_timestamp: lookback.items[index].system_timestamp,
                                    first_after_overflow: lookback.items[index]
                                        .first_after_overflow,
                                    raw_file_offset,
                                    raw_length: count,
                                    state: &lookback.items[index].state,
                                }
                                .write_to(index_file);
                                new_recording.size_bytes += count;
                                if let Err(error) = result {
                                    if !new_recording.index_file_error {
                                        new_recording.index_file_error = true;
                                        context.blocking_lock().shared_client_state.errors.push(
                                            format!(
                                                "Writing to \"{}\" failed ({})",
                                                index_file_path(
                                                    &new_recording.directory,
                                                    &new_recording.name,
                                                    true
                                                )
                                                .to_string_lossy(),
                                                error
                                            ),
                                        );
                                    }
                                }
                                if !std::rc::Rc::ptr_eq(
                                    &previous_configuration,
                                    &lookback.items[index].configuration,
                                ) {
                                    let configuration_string = configuration_update_to_toml(
                                        &lookback.items[index].datetime,
                                        &lookback.items[index].configuration,
                                    );
                                    let (count, result) = write_all_count(
                                        metadata_file,
                                        configuration_string.as_bytes(),
                                    );
                                    new_recording.size_bytes += count;
                                    if let Err(error) = result {
                                        if !new_recording.metadata_file_error {
                                            new_recording.metadata_file_error = true;
                                            context
                                                .blocking_lock()
                                                .shared_client_state
                                                .errors
                                                .push(format!(
                                                    "Writing to \"{}\" failed ({})",
                                                    metadata_file_path(
                                                        &new_recording.directory,
                                                        &new_recording.name,
                                                        true
                                                    )
                                                    .to_string_lossy(),
                                                    error
                                                ));
                                        }
                                    }
                                    previous_configuration =
                                        lookback.items[index].configuration.clone();
                                }
                                index = (index + 1) % lookback.items.len();
                            }
                        }
                    }
                }
            }
            *autostop_reference_t = adapter.current_t();
            Some(new_recording)
        }
        Err(error) => {
            context
                .blocking_lock()
                .shared_client_state
                .errors
                .push(format!(
                    "Creating \"{}\" failed ({})",
                    directory.join(name).to_string_lossy(),
                    error
                ));
            None
        }
    }
}

impl SamplerRecording {
    fn new(
        directory: std::path::PathBuf,
        name: String,
        format_type: u8,
    ) -> Result<SamplerRecording, std::io::Error> {
        let mut size_bytes = 0;
        let mut samples_file = std::io::BufWriter::new(std::fs::File::create(samples_file_path(
            &directory, &name, true,
        ))?);
        // 0 is the file version number (not a string terminator)
        samples_file.write_all(format!("{}\0", recordings::SAMPLES_FILE_SIGNATURE).as_bytes())?;
        samples_file.write_all(&[format_type])?;
        size_bytes += 16;
        Ok(SamplerRecording {
            directory,
            name,
            samples_file: Some(samples_file),
            samples_file_error: false,
            size_bytes,
        })
    }
}

impl Drop for SamplerRecording {
    fn drop(&mut self) {
        let _ = self.samples_file.take();
        let _ = std::fs::rename(
            samples_file_path(&self.directory, &self.name, true),
            samples_file_path(&self.directory, &self.name, false),
        );
    }
}

struct AutotriggerMovingWindow {
    log_values: [f64; constants::AUTOTRIGGER_MAXIMUM_WINDOW_SIZE],
    write_index: usize,
    actual_length: usize,
}

impl AutotriggerMovingWindow {
    fn new() -> Self {
        Self {
            log_values: [0.0; constants::AUTOTRIGGER_MAXIMUM_WINDOW_SIZE],
            write_index: 0,
            actual_length: 0,
        }
    }

    fn push(&mut self, value: f64) {
        self.log_values[self.write_index] = value.ln();
        self.write_index = (self.write_index + 1) % self.log_values.len();
        if self.actual_length < self.log_values.len() {
            self.actual_length += 1;
        }
    }

    fn mean(&self, length: usize) -> f64 {
        if self.actual_length == 0 {
            0.0
        } else {
            let length = length.min(self.actual_length);
            let mut index =
                (self.write_index + self.log_values.len() - length) % self.log_values.len();
            let mut total: f64 = 0.0;
            loop {
                total += self.log_values[index];
                index = (index + 1) % self.log_values.len();
                if index == self.write_index {
                    break;
                }
            }
            (total / length as f64).exp()
        }
    }
}

pub fn create_device_and_proxies(
    id: DeviceId,
    listed_device: neuromorphic_drivers::devices::ListedDevice,
    device: neuromorphic_drivers::Device,
    flag: neuromorphic_drivers::Flag<
        neuromorphic_drivers::Error,
        neuromorphic_drivers::UsbOverflow,
    >,
) -> (Device, DeviceSampler, DeviceProxy) {
    let record_configuration = std::sync::Arc::new(std::sync::Mutex::new(RecordConfiguration {
        action: RecordAction::Continue,
        lookback: protocol::Lookback::default(),
        autostop: protocol::Autostop::default(),
        autotrigger: protocol::Autotrigger::default(),
    }));
    let event_thread_state = std::sync::Arc::new(std::sync::Mutex::new(EventThreadState {
        on_event_rate: 0.0,
        off_event_rate: 0.0,
        rising_trigger_count: 0,
        falling_trigger_count: 0,
        lookback_state: None,
        file_state: None,
        autotrigger_state: None,
    }));
    let inner = std::sync::Arc::new(device);
    let running = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(true));
    let configuration_changed = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let properties = Properties {
        name: listed_device.device_type.name().to_owned(),
        serial: listed_device
            .serial
            .expect("Device::new is given a valid listed_device"),
        speed: listed_device.speed.to_string(),
        bus_number: listed_device.bus_number,
        address: listed_device.address,
    };
    (
        Device {
            id,
            properties: properties.clone(),
            inner: inner.clone(),
            flag,
            record_configuration: record_configuration.clone(),
            event_thread_state: event_thread_state.clone(),
            running: running.clone(),
            configuration_changed: configuration_changed.clone(),
        },
        DeviceSampler {
            id,
            inner: inner.clone(),
            event_thread_state: event_thread_state.clone(),
            running,
        },
        DeviceProxy {
            id,
            properties,
            inner,
            record_configuration,
            configuration_changed,
        },
    )
}

impl Device {
    pub fn run(self, context: std::sync::Arc<tokio::sync::Mutex<crate::Context>>) {
        let stream_id = StreamId::new(self.id, 0);
        let (time_reference, router, stack) = {
            let context_guard = context.blocking_lock();
            (
                context_guard.time_reference,
                context_guard.router.clone(),
                context_guard.packet_stack.clone(),
            )
        };
        let mut recording: Option<Recording> = None;
        let mut adapter = self.inner.create_adapter();
        let mut data_buffer = Vec::new();
        let mut data_buffer_start_state = adapter.state();
        let mut data_buffer_on_event_count = 0;
        let mut data_buffer_off_event_count = 0;
        let mut next_packet_index: u64 = 1;
        let mut data_buffer_end_t =
            (next_packet_index as f64 * 1e6 / constants::PACKET_FREQUENCY).round() as u64;

        let mut event_rate_on_samples = [0.0; constants::EVENT_RATE_SAMPLES];
        let mut event_rate_off_samples = [0.0; constants::EVENT_RATE_SAMPLES];
        let mut event_rate_index = 0;
        let mut on_event_rate = 0.0;
        let mut off_event_rate = 0.0;

        let mut lookback: Option<Lookback> = None;
        let mut autostop_reference_t: u64 = 0;
        let mut autotrigger_moving_window = AutotriggerMovingWindow::new();

        loop {
            // break on error
            if let Err(error) = self.flag.load_error() {
                self.running
                    .store(false, std::sync::atomic::Ordering::Relaxed);
                let mut context_guard = context.blocking_lock();
                let _ = context_guard.id_to_device.remove(&self.id);
                context_guard.update_shared_client_state_devices();
                context_guard
                    .shared_client_state
                    .errors
                    .push(format!("Device error: {}", error));
                break;
            }

            // read the next camera buffer
            let buffer_view = self
                .inner
                .next_with_timeout(&std::time::Duration::from_millis(100));

            // read the record configuration
            let now = chrono::Local::now();
            let (record_action, new_lookback, autostop, autotrigger) = {
                let mut record_configuration_guard = self
                    .record_configuration
                    .lock()
                    .expect("record configuration mutex is poisoned");
                let record_action = record_configuration_guard.action.clone();
                record_configuration_guard.action = RecordAction::Continue;
                (
                    record_action,
                    record_configuration_guard.lookback,
                    record_configuration_guard.autostop,
                    record_configuration_guard.autotrigger,
                )
            };

            // read the camera configuration if it changed
            let new_configuration = if self
                .configuration_changed
                .swap(false, std::sync::atomic::Ordering::AcqRel)
            {
                Some(self.inner.current_configuration())
            } else {
                None
            };

            // write configuration updates to the metadata file
            if let Some(recording) = recording.as_mut() {
                if let Some(metadata_file) = recording.metadata_file.as_mut() {
                    if let Some(new_configuration) = new_configuration.as_ref() {
                        let configuration_string =
                            configuration_update_to_toml(&chrono::Local::now(), new_configuration);
                        let (count, result) =
                            write_all_count(metadata_file, configuration_string.as_bytes());
                        recording.size_bytes += count;
                        if let Err(error) = result {
                            if !recording.metadata_file_error {
                                recording.metadata_file_error = true;
                                context
                                    .blocking_lock()
                                    .shared_client_state
                                    .errors
                                    .push(format!(
                                        "Writing to \"{}\" failed ({})",
                                        metadata_file_path(
                                            &recording.directory,
                                            &recording.name,
                                            true
                                        )
                                        .to_string_lossy(),
                                        error
                                    ));
                            }
                        }
                    }
                }
            }

            // update the lookback configuration if it changed
            if new_lookback.enabled {
                match lookback.as_mut() {
                    Some(lookback) => {
                        if lookback.maximum_duration_us != new_lookback.maximum_duration_us {
                            lookback.update_maximum_duration_us(new_lookback.maximum_duration_us);
                        }
                        if lookback.maximum_size_bytes != new_lookback.maximum_size_bytes {
                            lookback.update_maximum_size_bytes(new_lookback.maximum_size_bytes);
                        }
                    }
                    None => {
                        let _ = lookback.replace(Lookback::new(
                                new_lookback.maximum_duration_us,
                                new_lookback.maximum_size_bytes,
                                match self.inner.as_ref() {
                                    neuromorphic_drivers::Device::InivationDavis346(_) => {
                                        neuromorphic_drivers::inivation_davis346::DEFAULT_USB_CONFIGURATION.buffer_length
                                    },
                                    neuromorphic_drivers::Device::InivationDvxplorer(_) => {
                                        neuromorphic_drivers::inivation_davis346::DEFAULT_USB_CONFIGURATION.buffer_length
                                    },
                                    neuromorphic_drivers::Device::PropheseeEvk3Hd(_) => {
                                        neuromorphic_drivers::prophesee_evk3_hd::DEFAULT_USB_CONFIGURATION.buffer_length
                                    },
                                    neuromorphic_drivers::Device::PropheseeEvk4(_) => {
                                        neuromorphic_drivers::prophesee_evk4::DEFAULT_USB_CONFIGURATION.buffer_length
                                    },
                                },
                                adapter.state(),
                                self.inner.current_configuration(),
                            ));
                    }
                }
            } else {
                let _ = lookback.take();
            }

            // start a new recording (manual trigger)
            match record_action {
                RecordAction::Continue => {}
                RecordAction::Start(name) => {
                    let _ = recording.take();
                    if let Some(new_recording) = create_new_recording(
                        &lookback,
                        &now,
                        &name,
                        &adapter,
                        &self.properties,
                        &new_configuration,
                        &self.inner,
                        &self.event_thread_state,
                        &context,
                        &mut autostop_reference_t,
                        Trigger::Manual(crate::utc_string(&now)),
                    ) {
                        let _ = recording.replace(new_recording);
                    }
                }
                RecordAction::Stop => {
                    let _ = recording.take();
                }
            }

            // build 1/60s event packets and calculate event rates
            if let Some(buffer_view) = buffer_view {
                let system_time = buffer_view
                    .system_time
                    .duration_since(std::time::SystemTime::UNIX_EPOCH)
                    .unwrap_or(std::time::Duration::default())
                    .as_micros() as u64;
                let system_timestamp = buffer_view
                    .instant
                    .duration_since(time_reference)
                    .as_micros() as u64;
                let mut rising_trigger_count = 0;
                let mut falling_trigger_count = 0;
                let previous_state = adapter.state();
                let mut autotrigger_state: Option<AutotriggerState> = None;
                let mut buffer_view_offset = 0;
                loop {
                    let (events_lengths, position) = adapter.events_lengths_until(
                        &buffer_view.slice[buffer_view_offset..],
                        data_buffer_end_t,
                    );
                    data_buffer.extend(
                        &buffer_view.slice[buffer_view_offset..buffer_view_offset + position],
                    );
                    match events_lengths {
                        neuromorphic_drivers::adapters::EventsLengths::Davis346(events_lengths) => {
                            todo!()
                        }
                        neuromorphic_drivers::adapters::EventsLengths::Dvxplorer(
                            events_lengths,
                        ) => {
                            todo!()
                        }
                        neuromorphic_drivers::adapters::EventsLengths::Evt3(events_lengths) => {
                            data_buffer_on_event_count += events_lengths.on;
                            data_buffer_off_event_count += events_lengths.off;
                            rising_trigger_count += events_lengths.trigger_rising as u32;
                            falling_trigger_count += events_lengths.trigger_falling as u32;
                        }
                    }
                    if (buffer_view_offset + position) / 2 >= buffer_view.slice.len() / 2 {
                        break;
                    }
                    buffer_view_offset += position;

                    // update the event rate (graph display)
                    event_rate_on_samples[event_rate_index] =
                        data_buffer_on_event_count as f64 * constants::PACKET_FREQUENCY;
                    event_rate_off_samples[event_rate_index] =
                        data_buffer_off_event_count as f64 * constants::PACKET_FREQUENCY;
                    event_rate_index = (event_rate_index + 1) % constants::EVENT_RATE_SAMPLES;
                    {
                        on_event_rate = (event_rate_on_samples.iter().sum::<f64>()
                            / constants::EVENT_RATE_SAMPLES as f64)
                            as f32;
                        off_event_rate = (event_rate_off_samples.iter().sum::<f64>()
                            / constants::EVENT_RATE_SAMPLES as f64)
                            as f32;
                    }

                    // update the event rate (auto-trigger)
                    autotrigger_moving_window.push(
                        (data_buffer_on_event_count + data_buffer_off_event_count) as f64
                            * constants::PACKET_FREQUENCY,
                    );
                    {
                        let short_value =
                            autotrigger_moving_window.mean(autotrigger.short_sliding_window);
                        let long_value =
                            autotrigger_moving_window.mean(autotrigger.long_sliding_window);
                        let ratio = if long_value == 0.0 {
                            1.0
                        } else {
                            short_value / long_value
                        };
                        update_autotrigger_state(
                            &mut autotrigger_state,
                            AutotriggerState {
                                short_value: short_value as f32,
                                long_value: long_value as f32,
                                ratio: ratio as f32,
                                threshold: autotrigger.threshold,
                            },
                        );
                    }
                    data_buffer_on_event_count = 0;
                    data_buffer_off_event_count = 0;

                    // send data to the UI
                    {
                        let router_guard = router.read().expect("router mutex is poisoned");
                        if let Some(clients_ids_and_senders) = router_guard.get(&stream_id) {
                            for (_, sender) in clients_ids_and_senders {
                                let buffer =
                                    { stack.lock().expect("packet stack mutex is poisoned").pop() };
                                if let Some(mut buffer) = buffer {
                                    match data_buffer_start_state {
                                        neuromorphic_drivers::adapters::State::Davis346(state) => {
                                            todo!()
                                        }
                                        neuromorphic_drivers::adapters::State::Dvxplorer(state) => {
                                            todo!()
                                        }
                                        neuromorphic_drivers::adapters::State::Evt3(state) => {
                                            let data_length = data_buffer.len().min(
                                                constants::PACKET_MAXIMUM_LENGTH as usize - 50,
                                            );
                                            let total_length = data_length + 50;
                                            buffer.clear();
                                            buffer.reserve_exact(total_length);
                                            buffer.extend(&(total_length as u32).to_le_bytes()); // 4
                                            buffer.extend(&system_time.to_le_bytes()); // 8
                                            buffer.extend(&system_timestamp.to_le_bytes()); // 8
                                            buffer.extend(&state.t.to_le_bytes()); // 8
                                            buffer.extend(&state.overflows.to_le_bytes()); // 4
                                            buffer.extend(&state.previous_msb_t.to_le_bytes()); // 2
                                            buffer.extend(&state.previous_lsb_t.to_le_bytes()); // 2
                                            buffer.extend(&state.x.to_le_bytes()); // 2
                                            buffer.extend(&state.y.to_le_bytes()); // 2
                                            buffer.extend(&(state.polarity as u16).to_le_bytes()); // 2
                                            buffer.extend(&data_buffer_end_t.to_le_bytes()); // 8
                                            buffer.extend(&data_buffer[0..data_length]);
                                            if let Ok(permit) = sender.try_reserve() {
                                                permit.send(buffer);
                                            } else {
                                                stack
                                                    .lock()
                                                    .expect("packet stack mutex is poisoned")
                                                    .push(buffer);
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    data_buffer.clear();
                    data_buffer_start_state = adapter.state();
                    next_packet_index += 1;
                    data_buffer_end_t = (next_packet_index as f64 * 1e6
                        / constants::PACKET_FREQUENCY)
                        .round() as u64;
                }

                // start a new recording (auto-trigger)
                if let Some(autotrigger_state) = autotrigger_state.as_ref() {
                    if autotrigger.enabled && autotrigger_state.ratio >= autotrigger_state.threshold
                    {
                        if recording.is_none() {
                            if let Some(new_recording) = create_new_recording(
                                &lookback,
                                &now,
                                "",
                                &adapter,
                                &self.properties,
                                &new_configuration,
                                &self.inner,
                                &self.event_thread_state,
                                &context,
                                &mut autostop_reference_t,
                                Trigger::Auto(Autotrigger {
                                    timestamp: crate::utc_string(&now),
                                    short_sliding_window: autotrigger.short_sliding_window,
                                    long_sliding_window: autotrigger.long_sliding_window,
                                    threshold: autotrigger.threshold,
                                }),
                            ) {
                                let _ = recording.replace(new_recording);
                            }
                        } else {
                            autostop_reference_t = previous_state.current_t();
                        }
                    }
                }

                // write raw event data to the recording
                if let Some(recording) = recording.as_mut() {
                    if let Some(raw_file) = recording.raw_file.as_mut() {
                        if let Some(index_file) = recording.index_file.as_mut() {
                            let raw_file_offset = recording.raw_file_offset;
                            let (count, result) = write_all_count(raw_file, buffer_view.slice);
                            recording.raw_file_offset += count;
                            recording.size_bytes += count;
                            if let Err(error) = result {
                                if !recording.raw_file_error {
                                    recording.raw_file_error = true;
                                    context.blocking_lock().shared_client_state.errors.push(
                                        format!(
                                            "Writing to \"{}\" failed ({})",
                                            raw_file_path(
                                                &recording.directory,
                                                &recording.name,
                                                true
                                            )
                                            .to_string_lossy(),
                                            error
                                        ),
                                    );
                                }
                            }
                            let (count, result) = Index {
                                system_time,
                                system_timestamp,
                                first_after_overflow: buffer_view.first_after_overflow,
                                raw_file_offset,
                                raw_length: count,
                                state: &previous_state,
                            }
                            .write_to(index_file);
                            recording.size_bytes += count;
                            if let Err(error) = result {
                                if !recording.index_file_error {
                                    recording.index_file_error = true;
                                    context.blocking_lock().shared_client_state.errors.push(
                                        format!(
                                            "Writing to \"{}\" failed ({})",
                                            index_file_path(
                                                &recording.directory,
                                                &recording.name,
                                                true
                                            )
                                            .to_string_lossy(),
                                            error
                                        ),
                                    );
                                }
                            }
                        }
                    }
                }

                // push data to the lookback buffer
                if let Some(lookback) = lookback.as_mut() {
                    lookback.push(
                        now,
                        system_time,
                        system_timestamp,
                        buffer_view.first_after_overflow,
                        previous_state,
                        adapter.current_t(),
                        buffer_view.slice,
                        new_configuration,
                    );
                }

                // stop recording if auto-stop is enabled
                if autostop.enabled
                    && adapter.current_t() >= autostop_reference_t + autostop.duration_us
                {
                    let _ = recording.take();
                }

                // send data to the sampler thread
                {
                    let mut event_thread_state_guard = self
                        .event_thread_state
                        .lock()
                        .expect("event thread state mutex is poisoned");
                    event_thread_state_guard.lookback_state =
                        lookback.as_ref().map(|lookback| lookback.state());
                    event_thread_state_guard.on_event_rate = on_event_rate;
                    event_thread_state_guard.off_event_rate = off_event_rate;
                    event_thread_state_guard.rising_trigger_count += rising_trigger_count;
                    event_thread_state_guard.falling_trigger_count += falling_trigger_count;
                    if let Some(autotrigger_state) = autotrigger_state {
                        update_autotrigger_state(
                            &mut event_thread_state_guard.autotrigger_state,
                            autotrigger_state,
                        );
                    }
                    match recording.as_ref() {
                        Some(recording) => {
                            recording.update_file_state(
                                adapter.current_t(),
                                &mut event_thread_state_guard.file_state,
                            );
                        }
                        None => {
                            let _ = event_thread_state_guard.file_state.take();
                        }
                    }
                }
            }
        }
        let _ = router
            .write()
            .expect("router mutex is poisoned")
            .remove(&stream_id);
    }
}

enum SamplerRecordingAction {
    Continue {
        duration_us: u64,
        size_bytes: u64,
    },
    Start {
        directory: std::path::PathBuf,
        name: String,
        duration_us: u64,
        size_bytes: u64,
    },
    Stop,
}

impl DeviceSampler {
    pub fn run(self, context: std::sync::Arc<tokio::sync::Mutex<crate::Context>>) {
        let stream_id = StreamId::new(self.id, 1);
        let (time_reference, router, sample_stack, record_state_stack) = {
            let context_guard = context.blocking_lock();
            (
                context_guard.time_reference,
                context_guard.router.clone(),
                context_guard.sample_stack.clone(),
                context_guard.record_state_stack.clone(),
            )
        };
        let mut sampler_recording: Option<SamplerRecording> = None;
        let mut previous_autotrigger_state = AutotriggerState {
            short_value: 0.0,
            long_value: 0.0,
            ratio: 1.0,
            threshold: protocol::Autotrigger::default().threshold,
        };
        let mut sampler_lookback: Option<SamplerLookback> = None;
        let mut next_sample = std::time::Instant::now();
        while self.running.load(std::sync::atomic::Ordering::Relaxed) {
            let now = std::time::Instant::now();
            if now < next_sample {
                std::thread::sleep(next_sample - now);
                next_sample += constants::SAMPLING_PERIOD;
            } else {
                while now >= next_sample {
                    next_sample += constants::SAMPLING_PERIOD;
                }
            }

            // sample camera data
            let mut ui_sample = match self.inner.as_ref() {
                neuromorphic_drivers::Device::InivationDavis346(_) => {
                    todo!()
                }
                neuromorphic_drivers::Device::InivationDvxplorer(_) => {
                    todo!()
                }
                neuromorphic_drivers::Device::PropheseeEvk3Hd(_) => {
                    todo!()
                }
                neuromorphic_drivers::Device::PropheseeEvk4(device) => {
                    let illuminance = device.illuminance().unwrap_or(u32::MAX);
                    let temperature = device
                        .temperature_celsius()
                        .map_or(f32::NAN, |temperature_celsius| temperature_celsius.0);
                    UiSample::Evk4(Evk4UiSample {
                        system_time: 0,
                        system_timestamp: 0,
                        on_event_rate: 0.0,
                        off_event_rate: 0.0,
                        rising_trigger_count: 0,
                        falling_trigger_count: 0,
                        illuminance: (EVK4_ILLUMINANCE_ALPHA * illuminance as f64)
                            .powf(EVK4_ILLUMINANCE_BETA)
                            as f32,
                        temperature,
                        autotrigger_state: previous_autotrigger_state,
                    })
                }
            };

            // receive data from the event thread
            let (sampler_recording_action, lookback_state) = {
                let mut event_thread_state_guard = self
                    .event_thread_state
                    .lock()
                    .expect("event thread state mutex is poisoned");
                let system_time = std::time::SystemTime::now()
                    .duration_since(std::time::SystemTime::UNIX_EPOCH)
                    .unwrap_or(std::time::Duration::default())
                    .as_micros() as u64;
                let system_timestamp = std::time::Instant::now()
                    .duration_since(time_reference)
                    .as_micros() as u64;
                match &mut ui_sample {
                    UiSample::Davis346(davis346_ui_sample) => {
                        todo!()
                    }
                    UiSample::Evk3Hd(evk3_hd_ui_sample) => {
                        todo!()
                    }
                    UiSample::Evk4(evk4_ui_sample) => {
                        evk4_ui_sample.system_time = system_time;
                        evk4_ui_sample.system_timestamp = system_timestamp;
                        evk4_ui_sample.on_event_rate = event_thread_state_guard.on_event_rate;
                        evk4_ui_sample.off_event_rate = event_thread_state_guard.off_event_rate;
                        evk4_ui_sample.rising_trigger_count =
                            event_thread_state_guard.rising_trigger_count;
                        evk4_ui_sample.falling_trigger_count =
                            event_thread_state_guard.falling_trigger_count;
                        if let Some(autotrigger_state) =
                            event_thread_state_guard.autotrigger_state.take()
                        {
                            evk4_ui_sample.autotrigger_state = autotrigger_state;
                            previous_autotrigger_state = autotrigger_state;
                        }
                    }
                }
                event_thread_state_guard.rising_trigger_count = 0;
                event_thread_state_guard.falling_trigger_count = 0;
                let sampler_recording_action = match event_thread_state_guard.file_state.as_ref() {
                    Some(file_state) => match sampler_recording.as_ref() {
                        Some(sampler_recording) => {
                            if sampler_recording.directory == file_state.directory
                                && sampler_recording.name == file_state.name
                            {
                                SamplerRecordingAction::Continue {
                                    duration_us: file_state.duration_us,
                                    size_bytes: file_state.size_bytes,
                                }
                            } else {
                                SamplerRecordingAction::Start {
                                    directory: file_state.directory.clone(),
                                    name: file_state.name.clone(),
                                    duration_us: file_state.duration_us,
                                    size_bytes: file_state.size_bytes,
                                }
                            }
                        }
                        None => SamplerRecordingAction::Start {
                            directory: file_state.directory.clone(),
                            name: file_state.name.clone(),
                            duration_us: file_state.duration_us,
                            size_bytes: file_state.size_bytes,
                        },
                    },
                    None => SamplerRecordingAction::Stop,
                };
                (
                    sampler_recording_action,
                    event_thread_state_guard.lookback_state.clone(),
                )
            };

            // update the lookback configuration if needed
            match lookback_state.as_ref() {
                Some(lookback_state) => match sampler_lookback.as_mut() {
                    Some(sampler_lookback) => {
                        if sampler_lookback.maximum_duration_us
                            != lookback_state.maximum_duration_us
                        {
                            sampler_lookback
                                .update_maximum_duration_us(lookback_state.maximum_duration_us);
                        }
                    }
                    None => {
                        let _ = sampler_lookback.replace(SamplerLookback::new(
                            lookback_state.maximum_duration_us,
                            (&ui_sample).into(),
                        ));
                    }
                },
                None => {
                    let _ = sampler_lookback.take();
                }
            }

            // start a new recording (manual or auto trigger)
            let mut name_and_duration_us_and_size_bytes: Option<(String, u64, u64)> =
                match sampler_recording_action {
                    SamplerRecordingAction::Continue {
                        duration_us,
                        size_bytes,
                    } => match sampler_recording.as_ref() {
                        Some(sampler_recording) => {
                            Some((sampler_recording.name.clone(), duration_us, size_bytes))
                        }
                        None => unreachable!(),
                    },
                    SamplerRecordingAction::Start {
                        directory,
                        name,
                        duration_us,
                        size_bytes,
                    } => {
                        let _ = sampler_recording.take();
                        // 0 is Prophesee EVK4
                        match SamplerRecording::new(directory.clone(), name.clone(), 0) {
                            Ok(mut new_sampler_recording) => {
                                if let Some(samples_file) =
                                    new_sampler_recording.samples_file.as_mut()
                                {
                                    if let Some(sampler_lookback) = sampler_lookback.as_ref() {
                                        let mut index = sampler_lookback.read_index;
                                        let mut lookback_count = 0;
                                        let mut lookback_result = Ok(());
                                        while index != sampler_lookback.write_index {
                                            let (count, result) = sampler_lookback.items[index]
                                                .write_to(samples_file);
                                            lookback_count += count;
                                            if lookback_result.is_ok() && result.is_err() {
                                                lookback_result = result;
                                            }
                                            index = (index + 1) % sampler_lookback.items.len();
                                        }
                                        new_sampler_recording.size_bytes += lookback_count;
                                        if let Err(error) = lookback_result {
                                            if !new_sampler_recording.samples_file_error {
                                                new_sampler_recording.samples_file_error = true;
                                                context
                                                    .blocking_lock()
                                                    .shared_client_state
                                                    .errors
                                                    .push(format!(
                                                        "Writing to \"{}\" failed ({})",
                                                        samples_file_path(
                                                            &new_sampler_recording.directory,
                                                            &new_sampler_recording.name,
                                                            true
                                                        )
                                                        .to_string_lossy(),
                                                        error
                                                    ));
                                            }
                                        }
                                    }
                                }
                                let _ = sampler_recording.replace(new_sampler_recording);
                                Some((name, duration_us, size_bytes))
                            }
                            Err(error) => {
                                context
                                    .blocking_lock()
                                    .shared_client_state
                                    .errors
                                    .push(format!(
                                        "Creating \"{}\" failed ({})",
                                        samples_file_path(&directory, &name, true)
                                            .to_string_lossy(),
                                        error
                                    ));
                                Some((name, duration_us, size_bytes))
                            }
                        }
                    }
                    SamplerRecordingAction::Stop => {
                        let _ = sampler_recording.take();
                        None
                    }
                };

            // write data to the recording
            if let Some(sampler_recording) = sampler_recording.as_mut() {
                if let Some(samples_file) = sampler_recording.samples_file.as_mut() {
                    let file_sample: FileSample = (&ui_sample).into();
                    let (count, result) = file_sample.write_to(samples_file);
                    sampler_recording.size_bytes += count;
                    if let Err(error) = result {
                        if !sampler_recording.samples_file_error {
                            sampler_recording.samples_file_error = true;
                            context
                                .blocking_lock()
                                .shared_client_state
                                .errors
                                .push(format!(
                                    "Writing to \"{}\" failed ({})",
                                    samples_file_path(
                                        &sampler_recording.directory,
                                        &sampler_recording.name,
                                        true
                                    )
                                    .to_string_lossy(),
                                    error
                                ));
                        }
                    }
                    if let Some(name_and_duration_us_and_size_bytes) =
                        name_and_duration_us_and_size_bytes.as_mut()
                    {
                        name_and_duration_us_and_size_bytes.2 += sampler_recording.size_bytes;
                    }
                }
            }

            // send data to the UI
            {
                let router_guard = router.read().expect("router mutex is poisoned");
                if let Some(clients_ids_and_senders) = router_guard.get(&stream_id) {
                    for (_, sender) in clients_ids_and_senders {
                        let buffer = {
                            sample_stack
                                .lock()
                                .expect("packet stack mutex is poisoned")
                                .pop()
                        };
                        if let Some(mut buffer) = buffer {
                            let total_length = ui_sample.byte_length() + 4;
                            buffer.clear();
                            buffer.reserve_exact(total_length);
                            buffer.extend(&(total_length as u32).to_le_bytes()); // 4
                            ui_sample.serialize_to(&mut buffer);
                            if let Ok(permit) = sender.try_reserve() {
                                permit.send(buffer);
                            } else {
                                sample_stack
                                    .lock()
                                    .expect("sample stack mutex is poisoned")
                                    .push(buffer);
                            }
                        }
                    }
                }
                if let Some(clients_ids_and_senders) =
                    router_guard.get(&device::StreamId(constants::RECORD_STATE_STREAM_ID))
                {
                    for (_, sender) in clients_ids_and_senders {
                        let buffer = {
                            record_state_stack
                                .lock()
                                .expect("packet stack mutex is poisoned")
                                .pop()
                        };
                        if let Some(mut buffer) = buffer {
                            buffer.clear();
                            let total_length = 42
                                + name_and_duration_us_and_size_bytes
                                    .as_ref()
                                    .map_or(0, |(name, _, _)| name.len());
                            buffer.reserve_exact(total_length);
                            buffer.extend(&(total_length as u32).to_le_bytes()); // 4
                            serialize_record_state_to(
                                self.id,
                                &lookback_state,
                                &name_and_duration_us_and_size_bytes,
                                &mut buffer,
                            );
                            if let Ok(permit) = sender.try_reserve() {
                                permit.send(buffer);
                            } else {
                                sample_stack
                                    .lock()
                                    .expect("sample stack mutex is poisoned")
                                    .push(buffer);
                            }
                        }
                    }
                }
            }

            // push data to the lookback buffer
            if let Some(sampler_lookback) = sampler_lookback.as_mut() {
                sampler_lookback.push((&ui_sample).into());
            }
        }
        let _ = router
            .write()
            .expect("router mutex is poisoned")
            .remove(&stream_id);
    }
}
