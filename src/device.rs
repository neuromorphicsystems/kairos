use crate::constants;
use crate::Context;

use neuromorphic_drivers::UsbDevice;

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
}

#[derive(Default, PartialEq, Eq)]
pub struct Lookback {
    maximum_duration_us: u64,
    maximum_size_bytes: usize,
}

#[derive(Default, PartialEq, Eq)]
pub struct Autotrigger {}

#[derive(Default, PartialEq, Eq)]
pub struct RecordTarget {
    pub lookback: Option<Lookback>,
    pub autotrigger: Option<Autotrigger>,
    pub path: Option<std::path::PathBuf>,
}

#[derive(Default, PartialEq, Eq)]
struct SamplerRecordTarget {
    lookback: Option<u64>,
    path: Option<std::path::PathBuf>,
}

pub struct Device {
    id: DeviceId,
    bus_number: u8,
    address: u8,
    inner: std::sync::Arc<neuromorphic_drivers::Device>,
    flag:
        neuromorphic_drivers::Flag<neuromorphic_drivers::Error, neuromorphic_drivers::UsbOverflow>,
    record_target: std::sync::Arc<std::sync::Mutex<RecordTarget>>,
    packed_event_rate: std::sync::Arc<std::sync::atomic::AtomicU64>,
    running: std::sync::Arc<std::sync::atomic::AtomicBool>,
}

pub struct DeviceSampler {
    id: DeviceId,
    inner: std::sync::Arc<neuromorphic_drivers::Device>,
    record_target: std::sync::Arc<std::sync::Mutex<SamplerRecordTarget>>,
    packed_event_rate: std::sync::Arc<std::sync::atomic::AtomicU64>,
    running: std::sync::Arc<std::sync::atomic::AtomicBool>,
}

pub struct DeviceProxy {
    pub id: DeviceId,
    pub serial: String,
    pub speed: String,
    pub inner: std::sync::Arc<neuromorphic_drivers::Device>,
}

#[derive(Debug)]
pub struct Sample {
    id: DeviceId,
    system_time: u64,
    system_timestamp: u64,
    illuminance: u32,
    temperature: f32,
    on_event_rate: f32,
    off_event_rate: f32,
}

impl Device {
    pub fn new(
        id: DeviceId,
        listed_device: neuromorphic_drivers::devices::ListedDevice,
        device: neuromorphic_drivers::Device,
        flag: neuromorphic_drivers::Flag<
            neuromorphic_drivers::Error,
            neuromorphic_drivers::UsbOverflow,
        >,
    ) -> ((u8, u8), Device, DeviceSampler, DeviceProxy) {
        let inner = std::sync::Arc::new(device);
        let packed_event_rate = std::sync::Arc::new(std::sync::atomic::AtomicU64::new(0));
        let running = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(true));
        (
            (listed_device.bus_number, listed_device.address),
            Self {
                id,
                bus_number: listed_device.bus_number,
                address: listed_device.address,
                inner: inner.clone(),
                flag,
                record_target: std::sync::Arc::new(std::sync::Mutex::new(RecordTarget::default())),
                packed_event_rate: packed_event_rate.clone(),
                running: running.clone(),
            },
            DeviceSampler {
                id,
                inner: inner.clone(),
                record_target: std::sync::Arc::new(std::sync::Mutex::new(
                    SamplerRecordTarget::default(),
                )),
                packed_event_rate,
                running,
            },
            DeviceProxy {
                id,
                serial: listed_device
                    .serial
                    .expect("Device::new is given a valid listed_device"),
                speed: listed_device.speed.to_string(),
                inner,
            },
        )
    }

    pub fn run(self, context: std::sync::Arc<tokio::sync::Mutex<Context>>) {
        let stream_id = StreamId::new(self.id, 0);
        let (time_reference, router, stack) = {
            let context_guard = context.blocking_lock();
            (
                context_guard.time_reference,
                context_guard.router.clone(),
                context_guard.packet_stack.clone(),
            )
        };
        let mut record_target = RecordTarget::default();
        let mut record_file: Option<std::fs::File> = None;
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

        loop {
            if let Err(error) = self.flag.load_error() {
                println!("{error:?}"); // @DEV
                self.running
                    .store(false, std::sync::atomic::Ordering::Relaxed);
                let mut context_guard = context.blocking_lock();
                let _ = context_guard
                    .bus_number_and_address_to_device
                    .remove(&(self.bus_number, self.address));
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

                loop {
                    let (events_lengths, position) =
                        adapter.events_lengths_until(buffer_view.slice, data_buffer_end_t);
                    data_buffer.extend(buffer_view.slice);
                    match events_lengths {
                        neuromorphic_drivers::adapters::EventsLengths::Davis346(events_lengths) => {
                            data_buffer_on_event_count += events_lengths.on;
                            data_buffer_off_event_count += events_lengths.off;
                        }
                        neuromorphic_drivers::adapters::EventsLengths::Evt3(events_lengths) => {
                            data_buffer_on_event_count += events_lengths.on;
                            data_buffer_off_event_count += events_lengths.off;
                        }
                    }
                    if position / 2 < buffer_view.slice.len() / 2 {
                        event_rate_on_samples[event_rate_index] =
                            data_buffer_on_event_count as f64 * constants::PACKET_FREQUENCY;
                        event_rate_off_samples[event_rate_index] =
                            data_buffer_off_event_count as f64 * constants::PACKET_FREQUENCY;
                        event_rate_index = (event_rate_index + 1) % constants::EVENT_RATE_SAMPLES;
                        {
                            let on_event_rate = (event_rate_on_samples.iter().sum::<f64>()
                                / constants::EVENT_RATE_SAMPLES as f64)
                                as f32;
                            let off_event_rate = (event_rate_off_samples.iter().sum::<f64>()
                                / constants::EVENT_RATE_SAMPLES as f64)
                                as f32;
                            let mut packed_event_rate_bytes = [0u8; 8];
                            packed_event_rate_bytes[0..4]
                                .copy_from_slice(&on_event_rate.to_le_bytes());
                            packed_event_rate_bytes[4..8]
                                .copy_from_slice(&off_event_rate.to_le_bytes());
                            self.packed_event_rate.store(
                                u64::from_le_bytes(packed_event_rate_bytes),
                                std::sync::atomic::Ordering::Relaxed,
                            );
                        }
                        data_buffer_on_event_count = 0;
                        data_buffer_off_event_count = 0;

                        {
                            let router_guard = router.read().expect("router mutex is not poisoned");
                            if let Some(clients_ids_and_senders) = router_guard.get(&stream_id) {
                                for (client_id, sender) in clients_ids_and_senders {
                                    let buffer = {
                                        stack.lock().expect("packet stack is not poisoned").pop()
                                    };
                                    if let Some(mut buffer) = buffer {
                                        match data_buffer_start_state {
                                            neuromorphic_drivers::adapters::State::Davis346(
                                                state,
                                            ) => todo!(),
                                            neuromorphic_drivers::adapters::State::Evt3(state) => {
                                                let data_length = data_buffer.len().min(
                                                    constants::PACKET_MAXIMUM_LENGTH as usize - 34,
                                                );
                                                let total_length = data_length + 34;
                                                buffer.clear();
                                                buffer.reserve_exact(total_length);
                                                buffer.extend(&(total_length as u32).to_le_bytes()); // 4
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
                                                        .expect("packet stack is not poisoned")
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
    pub fn run(self, context: std::sync::Arc<tokio::sync::Mutex<Context>>) {
        let stream_id = StreamId::new(self.id, 1);
        let (time_reference, router, stack) = {
            let context_guard = context.blocking_lock();
            (
                context_guard.time_reference,
                context_guard.router.clone(),
                context_guard.sample_stack.clone(),
            )
        };
        let mut record_target = SamplerRecordTarget::default();
        let mut record_file: Option<std::fs::File> = None;
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
            let (illuminance, temperature) = match self.inner.as_ref() {
                neuromorphic_drivers::Device::InivationDavis346(_) => (u32::MAX, f32::NAN),
                neuromorphic_drivers::Device::PropheseeEvk3Hd(_) => (u32::MAX, f32::NAN),
                neuromorphic_drivers::Device::PropheseeEvk4(device) => {
                    let illuminance = device.illuminance().unwrap_or(u32::MAX);
                    let temperature = device
                        .temperature_celsius()
                        .map_or(f32::NAN, |temperature_celsius| temperature_celsius.0);
                    (illuminance, temperature)
                }
            };
            let packed_event_rate = self
                .packed_event_rate
                .load(std::sync::atomic::Ordering::Relaxed);
            let packed_event_rate_bytes = packed_event_rate.to_le_bytes();
            let system_time = std::time::SystemTime::now()
                .duration_since(std::time::SystemTime::UNIX_EPOCH)
                .unwrap_or(std::time::Duration::default())
                .as_micros() as u64;
            let system_timestamp = std::time::Instant::now()
                .duration_since(time_reference)
                .as_micros() as u64;
            let sample = Sample {
                id: self.id,
                system_time,
                system_timestamp,
                illuminance,
                temperature,
                on_event_rate: f32::from_le_bytes(
                    packed_event_rate_bytes[0..4].try_into().expect("4 bytes"),
                ),
                off_event_rate: f32::from_le_bytes(
                    packed_event_rate_bytes[4..8].try_into().expect("4 bytes"),
                ),
            };

            // @DEV send state on dedicated channel (no pull)
        }
        println!("sampler loop done"); // @DEV

        let _ = router
            .write()
            .expect("router mutex is not poisoned")
            .remove(&stream_id);
    }
}
