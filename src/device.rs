use crate::constants;
use crate::device;

use neuromorphic_drivers::UsbDevice;

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

    pub fn device_id(self) -> u32 {
        (self.0 & 0xFFFFFF00) >> 8
    }

    pub fn stream_index(self) -> u8 {
        (self.0 & 0xFF) as u8
    }
}

#[derive(Default, PartialEq)]
pub struct Lookback {
    maximum_duration_us: u64,
    maximum_size_bytes: usize,
}

#[derive(Default, PartialEq)]
pub struct Autotrigger {
    bin_duration_us: u64,
    sliding_window_bins: usize,
    threshold: f32,
}

#[derive(PartialEq)]
pub enum RecordAction {
    Continue,
    Start {
        directory: std::path::PathBuf,
        name: String,
    },
    Stop,
}

impl Default for RecordAction {
    fn default() -> Self {
        Self::Continue
    }
}

#[derive(Default)]
pub struct RecordConfiguration {
    pub autotrigger: Option<Autotrigger>,
    pub autostop_us: Option<u64>,
    pub lookback: Option<Lookback>,
    pub action: RecordAction,
}

#[derive(Default, Clone)]
struct LookbackBufferState {
    maximum_duration_us: u64,
    duration_us: u64,
    size_bytes: usize,
}

#[derive(Default, Clone)]
struct FileState {
    name: String,
    duration_us: u64,
    size_bytes: usize,
}

#[derive(Default)]
struct EventThreadState {
    on_event_rate: f32,
    off_event_rate: f32,
    rising_trigger_count: u32,
    falling_trigger_count: u32,
    lookback_buffer_state: Option<LookbackBufferState>,
    file_state: Option<FileState>,
    // @DEV add auto-trigger fields
}

pub struct Device {
    id: DeviceId,
    bus_number: u8,
    address: u8,
    inner: std::sync::Arc<neuromorphic_drivers::Device>,
    flag:
        neuromorphic_drivers::Flag<neuromorphic_drivers::Error, neuromorphic_drivers::UsbOverflow>,
    record_configuration: std::sync::Arc<std::sync::Mutex<RecordConfiguration>>,
    event_thread_state: std::sync::Arc<std::sync::Mutex<EventThreadState>>,
    running: std::sync::Arc<std::sync::atomic::AtomicBool>,
}

pub struct DeviceSampler {
    id: DeviceId,
    inner: std::sync::Arc<neuromorphic_drivers::Device>,
    event_thread_state: std::sync::Arc<std::sync::Mutex<EventThreadState>>,
    running: std::sync::Arc<std::sync::atomic::AtomicBool>,
}

pub struct DeviceProxy {
    pub id: DeviceId,
    pub bus_number: u8,
    pub address: u8,
    pub serial: String,
    pub speed: String,
    pub inner: std::sync::Arc<neuromorphic_drivers::Device>,
    pub configuration: neuromorphic_drivers::Configuration,
    pub record_configuration: std::sync::Arc<std::sync::Mutex<RecordConfiguration>>,
}

#[derive(Debug)]
struct Davis346Sample {
    system_time: u64,
    system_timestamp: u64,
    on_event_rate: f32,
    off_event_rate: f32,
}

#[derive(Debug)]
struct Evk3HdSample {
    system_time: u64,
    system_timestamp: u64,
    on_event_rate: f32,
    off_event_rate: f32,
}

#[derive(Debug)]
struct Evk4Sample {
    system_time: u64,
    system_timestamp: u64,
    on_event_rate: f32,
    off_event_rate: f32,
    rising_trigger_count: u32,
    falling_trigger_count: u32,
    illuminance: f32,
    temperature: f32,
}

fn serialize_state(state: &neuromorphic_drivers::adapters::State) {}

struct LookbackBufferItem {
    state: neuromorphic_drivers::adapters::State,
    end_t: u64,
    raw: Vec<u8>,
}

struct LookbackBuffer {
    maximum_duration_us: u64,
    maximum_size_bytes: usize,
    duration_us: u64,
    size_bytes: usize,
    read_index: usize,
    write_index: usize,
    items: Vec<LookbackBufferItem>,
}

impl LookbackBuffer {
    fn resize(&mut self, new_length: usize) {
        let mut new_items = Vec::with_capacity(new_length);
        let count = (new_length - 1)
            .min((self.write_index + self.items.len() - self.read_index) % self.items.len());
        let mut index = (self.write_index + self.items.len() - count) % self.items.len();
        while index != self.write_index {
            let mut new_item = LookbackBufferItem {
                state: self.items[index].state,
                end_t: self.items[index].end_t,
                raw: Vec::new(),
            };
            std::mem::swap(&mut self.items[index].raw, &mut new_item.raw);
            new_items.push(new_item);
            index = (index + 1) % self.items.len();
        }
        std::mem::swap(&mut self.items, &mut new_items);
    }

    fn push(&mut self, state: &neuromorphic_drivers::adapters::State, end_t: u64, buffer: &[u8]) {
        self.items[self.write_index].state = state.clone();
        self.items[self.write_index].end_t = end_t;
        self.items[self.write_index].raw.resize(buffer.len(), 0);
        self.items[self.write_index].raw.copy_from_slice(buffer);
        self.size_bytes += buffer.len();
        self.write_index = (self.write_index + 1) % self.items.len();
        if self.write_index == self.read_index {
            self.read_index = (self.read_index + 1) % self.items.len();
        }
        while (self.read_index + 1) % self.items.len() != self.write_index {
            if end_t - self.items[self.read_index].end_t < self.maximum_duration_us {
                break;
            }
            self.size_bytes -= self.items[self.read_index].raw.len();
            self.read_index = (self.read_index + 1) % self.items.len();
        }
        self.duration_us = end_t - self.items[self.read_index].end_t;
    }

    fn state(&self) -> LookbackBufferState {
        LookbackBufferState {
            maximum_duration_us: self.maximum_duration_us,
            duration_us: self.duration_us,
            size_bytes: self.size_bytes,
        }
    }
}

#[derive(Debug)]
enum Sample {
    Davis346(Davis346Sample),
    Evk3HdSample(Evk3HdSample),
    Evk4Sample(Evk4Sample),
}

impl Sample {
    fn byte_length(&self) -> usize {
        match self {
            Sample::Davis346(_) => 24,
            Sample::Evk3HdSample(_) => 24,
            Sample::Evk4Sample(_) => 40,
        }
    }

    fn serialize_to(&self, buffer: &mut Vec<u8>) {
        match self {
            Sample::Davis346(davis346_sample) => {
                buffer.extend(&davis346_sample.system_time.to_le_bytes()); // 8
                buffer.extend(&davis346_sample.system_timestamp.to_le_bytes()); // 8
                buffer.extend(&davis346_sample.on_event_rate.to_le_bytes()); // 4
                buffer.extend(&davis346_sample.off_event_rate.to_le_bytes()); // 4
            }
            Sample::Evk3HdSample(evk3_hd_sample) => {
                buffer.extend(&evk3_hd_sample.system_time.to_le_bytes()); // 8
                buffer.extend(&evk3_hd_sample.system_timestamp.to_le_bytes()); // 8
                buffer.extend(&evk3_hd_sample.on_event_rate.to_le_bytes()); // 4
                buffer.extend(&evk3_hd_sample.off_event_rate.to_le_bytes()); // 4
            }
            Sample::Evk4Sample(evk4_sample) => {
                buffer.extend(&evk4_sample.system_time.to_le_bytes()); // 8
                buffer.extend(&evk4_sample.system_timestamp.to_le_bytes()); // 8
                buffer.extend(&evk4_sample.on_event_rate.to_le_bytes()); // 4
                buffer.extend(&evk4_sample.off_event_rate.to_le_bytes()); // 4
                buffer.extend(&evk4_sample.rising_trigger_count.to_le_bytes()); // 4
                buffer.extend(&evk4_sample.falling_trigger_count.to_le_bytes()); // 4
                buffer.extend(&evk4_sample.illuminance.to_le_bytes()); // 4
                buffer.extend(&evk4_sample.temperature.to_le_bytes()); // 4
            }
        }
    }
}

fn serialize_record_state_to(
    lookback_buffer_state: &Option<LookbackBufferState>,
    file_state: &Option<FileState>,
    buffer: &mut Vec<u8>,
) {
    match lookback_buffer_state {
        Some(lookback_buffer_state) => {
            buffer.push(1); // 1
            buffer.extend_from_slice(&lookback_buffer_state.duration_us.to_le_bytes()); // 8
            buffer.extend_from_slice(&lookback_buffer_state.size_bytes.to_le_bytes());
            // 8
        }
        None => {
            buffer.push(0); // 1
            buffer.extend_from_slice(&(0u64).to_le_bytes()); // 8
            buffer.extend_from_slice(&(0u64).to_le_bytes()); // 8
        }
    }
    match file_state {
        Some(file_state) => {
            buffer.push(1); // 1
            buffer.extend_from_slice(&file_state.duration_us.to_le_bytes()); // 8
            buffer.extend_from_slice(&file_state.size_bytes.to_le_bytes()); // 8
            buffer.extend_from_slice(file_state.name.as_bytes());
        }
        None => {
            buffer.push(0); // 1
            buffer.extend_from_slice(&(0u64).to_le_bytes()); // 8
            buffer.extend_from_slice(&(0u64).to_le_bytes()); // 8
        }
    }
}

impl Device {
    pub fn create_device_and_proxies(
        id: DeviceId,
        listed_device: neuromorphic_drivers::devices::ListedDevice,
        device: neuromorphic_drivers::Device,
        flag: neuromorphic_drivers::Flag<
            neuromorphic_drivers::Error,
            neuromorphic_drivers::UsbOverflow,
        >,
    ) -> (Device, DeviceSampler, DeviceProxy) {
        let configuration = device.default_configuration();
        let record_configuration =
            std::sync::Arc::new(std::sync::Mutex::new(RecordConfiguration::default()));
        let event_thread_state =
            std::sync::Arc::new(std::sync::Mutex::new(EventThreadState::default()));
        let inner = std::sync::Arc::new(device);
        let running = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(true));
        (
            Self {
                id,
                bus_number: listed_device.bus_number,
                address: listed_device.address,
                inner: inner.clone(),
                flag,
                record_configuration: record_configuration.clone(),
                event_thread_state: event_thread_state.clone(),
                running: running.clone(),
            },
            DeviceSampler {
                id,
                inner: inner.clone(),
                event_thread_state,
                running,
            },
            DeviceProxy {
                id,
                bus_number: listed_device.bus_number,
                address: listed_device.address,
                serial: listed_device
                    .serial
                    .expect("Device::new is given a valid listed_device"),
                speed: listed_device.speed.to_string(),
                inner,
                configuration,
                record_configuration,
            },
        )
    }

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
        let mut record_configuration = RecordConfiguration::default();
        let mut raw_file: Option<std::io::BufWriter<std::fs::File>> = None;
        let mut index_file: Option<std::io::BufWriter<std::fs::File>> = None;
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

        let mut lookback_buffer: Option<LookbackBuffer> = None;

        loop {
            if let Err(error) = self.flag.load_error() {
                println!("device error: {error:?}"); // @DEV
                self.running
                    .store(false, std::sync::atomic::Ordering::Relaxed);
                let mut context_guard = context.blocking_lock();
                let _ = context_guard.id_to_device.remove(&self.id);
                context_guard.update_shared_client_state_devices();
                break;
            }
            if let Some(buffer_view) = self
                .inner
                .next_with_timeout(&std::time::Duration::from_millis(100))
            {
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
                loop {
                    let (events_lengths, position) =
                        adapter.events_lengths_until(buffer_view.slice, data_buffer_end_t);
                    data_buffer.extend(buffer_view.slice);
                    match events_lengths {
                        neuromorphic_drivers::adapters::EventsLengths::Davis346(events_lengths) => {
                            data_buffer_on_event_count += events_lengths.on;
                            data_buffer_off_event_count += events_lengths.off;
                            rising_trigger_count += events_lengths.trigger as u32;
                        }
                        neuromorphic_drivers::adapters::EventsLengths::Evt3(events_lengths) => {
                            data_buffer_on_event_count += events_lengths.on;
                            data_buffer_off_event_count += events_lengths.off;
                            rising_trigger_count += events_lengths.trigger_rising as u32;
                            falling_trigger_count += events_lengths.trigger_falling as u32;
                        }
                    }
                    if position / 2 < buffer_view.slice.len() / 2 {
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
                        data_buffer_on_event_count = 0;
                        data_buffer_off_event_count = 0;
                        {
                            let router_guard = router.read().expect("router mutex is not poisoned");
                            if let Some(clients_ids_and_senders) = router_guard.get(&stream_id) {
                                for (_, sender) in clients_ids_and_senders {
                                    let buffer = {
                                        stack
                                            .lock()
                                            .expect("packet stack mutex is not poisoned")
                                            .pop()
                                    };
                                    if let Some(mut buffer) = buffer {
                                        match data_buffer_start_state {
                                            neuromorphic_drivers::adapters::State::Davis346(
                                                state,
                                            ) => todo!(),
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
                                                buffer
                                                    .extend(&(state.polarity as u16).to_le_bytes()); // 2
                                                buffer.extend(&data_buffer_end_t.to_le_bytes()); // 8
                                                buffer.extend(&data_buffer[0..data_length]);
                                                if let Ok(permit) = sender.try_reserve() {
                                                    permit.send(buffer);
                                                } else {
                                                    stack
                                                        .lock()
                                                        .expect(
                                                            "packet stack mutex is not poisoned",
                                                        )
                                                        .push(buffer);
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }

                        // @DEV copy state + data

                        data_buffer.clear();
                        data_buffer_start_state = adapter.state();
                        next_packet_index += 1;
                        data_buffer_end_t = (next_packet_index as f64 * 1e6
                            / constants::PACKET_FREQUENCY)
                            .round() as u64;
                    } else {
                        break;
                    }
                }
                {
                    let mut event_thread_state_guard = self
                        .event_thread_state
                        .lock()
                        .expect("event thread state mutex is not poisoned");
                    event_thread_state_guard.lookback_buffer_state = lookback_buffer
                        .as_ref()
                        .map(|lookback_buffer| lookback_buffer.state());
                    //event_thread_state_guard.path: Option<std::path::PathBuf>,
                    event_thread_state_guard.on_event_rate = on_event_rate;
                    event_thread_state_guard.off_event_rate = off_event_rate;
                    event_thread_state_guard.rising_trigger_count += rising_trigger_count;
                    event_thread_state_guard.falling_trigger_count += falling_trigger_count;
                    // @TODO event_thread_state_guard.recording_duration_us;
                    // @TODO event_thread_state_guard.recording_size_bytes;
                }
            }
        }

        println!("main loop done"); // @DEV

        let _ = router
            .write()
            .expect("router mutex is not poisoned")
            .remove(&stream_id);
    }
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
        let mut samples_file: Option<std::io::BufWriter<std::fs::File>> = None;
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
            let mut sample = match self.inner.as_ref() {
                neuromorphic_drivers::Device::InivationDavis346(_) => {
                    Sample::Davis346(Davis346Sample {
                        system_time: 0,
                        system_timestamp: 0,
                        on_event_rate: 0.0,
                        off_event_rate: 0.0,
                    })
                }
                neuromorphic_drivers::Device::PropheseeEvk3Hd(_) => {
                    Sample::Evk3HdSample(Evk3HdSample {
                        system_time: 0,
                        system_timestamp: 0,
                        on_event_rate: 0.0,
                        off_event_rate: 0.0,
                    })
                }
                neuromorphic_drivers::Device::PropheseeEvk4(device) => {
                    let illuminance = device.illuminance().unwrap_or(u32::MAX);
                    let temperature = device
                        .temperature_celsius()
                        .map_or(f32::NAN, |temperature_celsius| temperature_celsius.0);
                    Sample::Evk4Sample(Evk4Sample {
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
                    })
                }
            };
            let (lookback_buffer_state, file_state) = {
                let mut event_thread_state_guard = self
                    .event_thread_state
                    .lock()
                    .expect("event thread state mutex is not poisoned");
                let system_time = std::time::SystemTime::now()
                    .duration_since(std::time::SystemTime::UNIX_EPOCH)
                    .unwrap_or(std::time::Duration::default())
                    .as_micros() as u64;
                let system_timestamp = std::time::Instant::now()
                    .duration_since(time_reference)
                    .as_micros() as u64;
                match &mut sample {
                    Sample::Davis346(davis346_sample) => {
                        davis346_sample.system_time = system_time;
                        davis346_sample.system_timestamp = system_timestamp;
                        davis346_sample.on_event_rate = event_thread_state_guard.on_event_rate;
                        davis346_sample.off_event_rate = event_thread_state_guard.off_event_rate;
                    }
                    Sample::Evk3HdSample(evk3_hd_sample) => {
                        evk3_hd_sample.system_time = system_time;
                        evk3_hd_sample.system_timestamp = system_timestamp;
                        evk3_hd_sample.on_event_rate = event_thread_state_guard.on_event_rate;
                        evk3_hd_sample.off_event_rate = event_thread_state_guard.off_event_rate;
                    }
                    Sample::Evk4Sample(evk4_sample) => {
                        evk4_sample.system_time = system_time;
                        evk4_sample.system_timestamp = system_timestamp;
                        evk4_sample.on_event_rate = event_thread_state_guard.on_event_rate;
                        evk4_sample.off_event_rate = event_thread_state_guard.off_event_rate;
                        evk4_sample.rising_trigger_count =
                            event_thread_state_guard.rising_trigger_count;
                        evk4_sample.falling_trigger_count =
                            event_thread_state_guard.falling_trigger_count;
                    }
                }
                event_thread_state_guard.rising_trigger_count = 0;
                event_thread_state_guard.falling_trigger_count = 0;
                (
                    event_thread_state_guard.lookback_buffer_state.clone(),
                    event_thread_state_guard.file_state.clone(),
                )
            };
            {
                let router_guard = router.read().expect("router mutex is not poisoned");
                if let Some(clients_ids_and_senders) = router_guard.get(&stream_id) {
                    for (_, sender) in clients_ids_and_senders {
                        let buffer = {
                            sample_stack
                                .lock()
                                .expect("packet stack mutex is not poisoned")
                                .pop()
                        };
                        if let Some(mut buffer) = buffer {
                            let total_length = sample.byte_length() + 4;
                            buffer.clear();
                            buffer.reserve_exact(total_length);
                            buffer.extend(&(total_length as u32).to_le_bytes()); // 4
                            sample.serialize_to(&mut buffer);
                            if let Ok(permit) = sender.try_reserve() {
                                permit.send(buffer);
                            } else {
                                sample_stack
                                    .lock()
                                    .expect("sample stack mutex is not poisoned")
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
                                .expect("packet stack mutex is not poisoned")
                                .pop()
                        };
                        if let Some(mut buffer) = buffer {
                            buffer.clear();
                            let total_length = 38
                                + file_state
                                    .as_ref()
                                    .map_or(0, |file_state| file_state.name.len());
                            buffer.reserve_exact(total_length);
                            buffer.extend(&(total_length as u32).to_le_bytes()); // 4
                            serialize_record_state_to(
                                &lookback_buffer_state,
                                &file_state,
                                &mut buffer,
                            );
                            if let Ok(permit) = sender.try_reserve() {
                                permit.send(buffer);
                            } else {
                                sample_stack
                                    .lock()
                                    .expect("sample stack mutex is not poisoned")
                                    .push(buffer);
                            }
                        }
                    }
                }
            }
        }
        println!("sampler loop done"); // @DEV

        let _ = router
            .write()
            .expect("router mutex is not poisoned")
            .remove(&stream_id);
    }
}
