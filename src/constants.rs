#[allow(dead_code)]
pub const UNIDIRECTIONAL_STREAMS_ID: u32 = 0xfffff0; // reserved for the UI
#[allow(dead_code)]
pub const BIDIRECTIONAL_STREAMS_ID: u32 = 0xfffff1; // reserved for the UI
pub const MESSAGE_STREAM_ID: u32 = 0xffffff;
pub const MESSAGE_MAXIMUM_LENGTH: u32 = 1 << 20;
pub const MESSAGE_RECOMMENDED_BUFFER_COUNT: u32 = 32;
pub const PACKET_MAXIMUM_LENGTH: u32 = 1 << 22;
pub const PACKET_RECOMMENDED_BUFFER_COUNT: u32 = 16;
pub const PACKET_FREQUENCY: f64 = 60.0;
pub const EVENT_RATE_SAMPLES: usize = 6;
pub const SAMPLING_PERIOD: std::time::Duration = std::time::Duration::from_millis(100);
pub const STACK_MINIMUM_TIME_WINDOW: std::time::Duration =  std::time::Duration::from_secs(1);
pub const STACK_MINIMUM_SAMPLES: usize = 10;
pub const SAMPLE_STACK_LENGTH: usize = 256;
