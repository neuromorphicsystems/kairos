mod client;
mod constants;
mod device;
mod protocol;
mod stack;

use clap::Parser;

fn utc_string() -> String {
    chrono::Local::now()
        .naive_utc()
        .format("%F %T%.3f")
        .to_string()
}

fn utc_string_path_safe() -> String {
    chrono::Local::now()
        .naive_utc()
        .format("%FT%H-%M-%S%.6fZ")
        .to_string()
}

fn data_directory_default_value() -> std::ffi::OsString {
    match std::env::var("HOME") {
        Ok(home) => std::path::PathBuf::from(home)
            .join("kairos-data")
            .into_os_string(),
        _ => match std::env::current_dir() {
            Ok(current_dir) => current_dir.join("kairos-data").into_os_string(),
            _ => std::path::PathBuf::from("kairos-data").into_os_string(),
        },
    }
}

#[derive(clap::Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(short = 'p', long, default_value_t = 3000)]
    http_port: u16,

    #[arg(short = 'q', long, default_value_t = 3001)]
    transport_port: u16,

    #[arg(short = 'c', long, default_value_t = 60)]
    maximum_client_buffer_count: usize,

    #[arg(short = 's', long, default_value_t = 1usize << 30)]
    maximum_clients_buffering_memory: usize,

    #[arg(short = 'd', long, default_value = data_directory_default_value())]
    data_directory: std::path::PathBuf,
}

#[derive(Clone)]
struct Endpoint {
    port: u16,
    hash: String,
    created: std::time::SystemTime,
    terminate: std::sync::Arc<tokio::sync::Notify>,
    terminated: std::sync::Arc<tokio::sync::Notify>,
}

impl Endpoint {
    fn new(
        port: u16,
        identity: &wtransport::Identity,
    ) -> Result<Self, wtransport::tls::error::InvalidSan> {
        Ok(Self {
            port,
            hash: identity.certificate_chain().as_slice()[0]
                .hash()
                .fmt(wtransport::tls::Sha256DigestFmt::DottedHex),
            created: std::time::SystemTime::now(),
            terminate: std::sync::Arc::new(tokio::sync::Notify::new()),
            terminated: std::sync::Arc::new(tokio::sync::Notify::new()),
        })
    }
}

type Router = std::collections::HashMap<
    device::StreamId,
    Vec<(client::ClientId, tokio::sync::mpsc::Sender<Vec<u8>>)>,
>;

struct Context {
    time_reference: std::time::Instant,
    host_to_endpoint: std::collections::HashMap<String, Endpoint>,
    next_transport_port: u16,
    maximum_client_buffer_count: usize,
    next_client_id: client::ClientId,
    id_to_client: std::collections::HashMap<client::ClientId, client::ClientProxy>,
    shared_client_state: protocol::SharedClientState,
    id_to_device: std::collections::HashMap<device::DeviceId, device::DeviceProxy>,
    router: std::sync::Arc<std::sync::RwLock<Router>>,
    packet_stack: std::sync::Arc<std::sync::Mutex<stack::Stack>>,
    sample_stack: std::sync::Arc<std::sync::Mutex<stack::Stack>>,
    record_state_stack: std::sync::Arc<std::sync::Mutex<stack::Stack>>,
}

impl Context {
    fn broadcast_shared_client_state(&mut self) -> Result<(), anyhow::Error> {
        let bytes = self.shared_client_state.to_bytes()?;
        // we use a list of 'mpsc' channels instead of a single 'broadcast' channel
        // to avoid stalls
        for (_, client) in self.id_to_client.iter_mut() {
            let _ = client.shared_client_state_sender.send(bytes.clone());
        }
        Ok(())
    }

    fn update_shared_client_state_devices(&mut self) {
        let mut devices: Vec<_> = self
            .id_to_device
            .iter()
            .map(|(_, device)| protocol::Device {
                id: device.id.0,
                name: device.inner.name().to_owned(),
                serial: device.serial.clone(),
                speed: device.speed.to_string(),
                bus_number: device.bus_number,
                address: device.address,
                streams: match *device.inner {
                    neuromorphic_drivers::Device::InivationDavis346(_) => Vec::new(),
                    neuromorphic_drivers::Device::PropheseeEvk3Hd(_) => {
                        use neuromorphic_drivers::devices::prophesee_evk3_hd;
                        vec![protocol::Stream::Evt3 {
                            width: prophesee_evk3_hd::PROPERTIES.width,
                            height: prophesee_evk3_hd::PROPERTIES.height,
                        }]
                    }
                    neuromorphic_drivers::Device::PropheseeEvk4(_) => {
                        use neuromorphic_drivers::devices::prophesee_evk4;
                        vec![
                            protocol::Stream::Evt3 {
                                width: prophesee_evk4::PROPERTIES.width,
                                height: prophesee_evk4::PROPERTIES.height,
                            },
                            protocol::Stream::Evk4Samples,
                        ]
                    }
                },
                configuration: device.configuration.clone(),
            })
            .collect();
        std::mem::swap(&mut self.shared_client_state.devices, &mut devices);
        if let Err(error) = self.broadcast_shared_client_state() {
            println!("broadcast_shared_client_state error: {error:?}");
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let time_reference = std::time::Instant::now();
    let args = Args::parse();
    println!(
        "{} | Listening for HTTP requests on port {}",
        utc_string(),
        args.http_port
    );
    let tcp_listener =
        tokio::net::TcpListener::bind((std::net::Ipv4Addr::new(0, 0, 0, 0), args.http_port))
            .await?;
    let context = std::sync::Arc::new(tokio::sync::Mutex::new(Context {
        time_reference,
        host_to_endpoint: std::collections::HashMap::new(),
        next_transport_port: args.transport_port,
        maximum_client_buffer_count: args.maximum_client_buffer_count,
        next_client_id: client::ClientId(0),
        id_to_client: std::collections::HashMap::new(),
        shared_client_state: protocol::SharedClientState {
            data_directory: args.data_directory.to_string_lossy().to_string(),
            disk_available_and_total_space: None,
            devices: Vec::new(),
            errors: Vec::new(),
        },
        id_to_device: std::collections::HashMap::new(),
        router: std::sync::Arc::new(std::sync::RwLock::new(std::collections::HashMap::from([(
            device::StreamId(constants::RECORD_STATE_STREAM_ID),
            Vec::new(),
        )]))),
        packet_stack: std::sync::Arc::new(std::sync::Mutex::new(stack::Stack::new(
            args.maximum_clients_buffering_memory / constants::PACKET_MAXIMUM_LENGTH as usize,
            constants::STACK_MINIMUM_TIME_WINDOW,
            constants::STACK_MINIMUM_SAMPLES,
        ))),
        sample_stack: std::sync::Arc::new(std::sync::Mutex::new(stack::Stack::new(
            constants::SAMPLE_STACK_LENGTH,
            constants::STACK_MINIMUM_TIME_WINDOW,
            constants::STACK_MINIMUM_SAMPLES,
        ))),
        record_state_stack: std::sync::Arc::new(std::sync::Mutex::new(stack::Stack::new(
            constants::RECORD_STATE_STACK_LENGTH,
            constants::STACK_MINIMUM_TIME_WINDOW,
            constants::STACK_MINIMUM_SAMPLES,
        ))),
    }));

    {
        let context = context.clone();
        tokio::task::spawn_blocking(move || {
            let mut next_device_id = 0;
            loop {
                if let Ok(listed_devices) = neuromorphic_drivers::list_devices() {
                    let new_listed_devices: Vec<_> = {
                        let context_guard = context.blocking_lock();
                        listed_devices
                            .into_iter()
                            .filter(|device| {
                                device.serial.is_ok()
                                    && context_guard.id_to_device.iter().all(
                                        |(_, listed_device)| {
                                            device.bus_number != listed_device.bus_number
                                                || device.address != listed_device.address
                                        },
                                    )
                            })
                            .collect()
                    };
                    if !new_listed_devices.is_empty() {
                        let mut devices_and_proxies = Vec::with_capacity(new_listed_devices.len());
                        for listed_device in new_listed_devices {
                            if let Ok((flag, event_loop)) =
                                neuromorphic_drivers::flag_and_event_loop()
                            {
                                if let Ok(device) =
                                    listed_device.open(None, None, event_loop, flag.clone())
                                {
                                    println!(
                                        "{} | new device, id {}",
                                        utc_string(),
                                        next_device_id
                                    ); // @DEV
                                    devices_and_proxies.push(
                                        device::Device::create_device_and_proxies(
                                            device::DeviceId(next_device_id),
                                            listed_device,
                                            device,
                                            flag,
                                        ),
                                    );
                                    next_device_id = (next_device_id + 1) % 0x1000000;
                                }
                            }
                        }
                        let mut context_guard = context.blocking_lock();
                        for (device, device_sampler, device_proxy) in devices_and_proxies {
                            {
                                let mut router_guard = context_guard
                                    .router
                                    .write()
                                    .expect("router mutex is not poisoned");
                                router_guard
                                    .insert(device::StreamId::new(device_proxy.id, 0), Vec::new());
                                router_guard
                                    .insert(device::StreamId::new(device_proxy.id, 1), Vec::new());
                            }
                            {
                                let context = context.clone();
                                tokio::task::spawn_blocking(move || {
                                    device.run(context);
                                });
                            }
                            {
                                let context = context.clone();
                                tokio::task::spawn_blocking(move || {
                                    device_sampler.run(context);
                                });
                            }
                            context_guard
                                .id_to_device
                                .insert(device_proxy.id, device_proxy);
                        }
                        context_guard.update_shared_client_state_devices();
                    }
                }
                std::thread::sleep(std::time::Duration::from_secs(3));
            }
        });
    }

    // periodically check whether the (buffer) stacks can be shrunk
    {
        let (packet_stack, sample_stack, record_state_stack) = {
            let context_guard = context.lock().await;
            (
                context_guard.packet_stack.clone(),
                context_guard.sample_stack.clone(),
                context_guard.record_state_stack.clone(),
            )
        };
        tokio::spawn(async move {
            loop {
                packet_stack
                    .lock()
                    .expect("packet stack mutex is not poisoned")
                    .shrink_unused();
                sample_stack
                    .lock()
                    .expect("sample stack mutex is not poisoned")
                    .shrink_unused();
                record_state_stack
                    .lock()
                    .expect("sample stack mutex is not poisoned")
                    .shrink_unused();
                tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            }
        });
    }

    // periodically read the amount of space left on the device
    {
        let context = context.clone();
        tokio::spawn(async move {
            let mut disks = sysinfo::Disks::new();
            let mut disk_available_and_total_space: Option<(u64, u64)> = None;
            loop {
                let (data_directory, previous_disk_available_and_total_space) = {
                    let context_guard = context.lock().await;
                    (
                        std::path::PathBuf::from(&context_guard.shared_client_state.data_directory),
                        context_guard
                            .shared_client_state
                            .disk_available_and_total_space
                            .clone(),
                    )
                };
                disks.refresh_specifics(true, sysinfo::DiskRefreshKind::nothing().with_storage());
                #[cfg(any(target_os = "linux", target_os = "macos"))]
                {
                    #[cfg(target_os = "linux")]
                    use std::os::linux::fs::MetadataExt;
                    #[cfg(target_os = "macos")]
                    use std::os::macos::fs::MetadataExt;
                    if let Ok(metadata) = std::fs::metadata(&data_directory) {
                        let data_directory_device_id = metadata.st_dev();
                        let mut found = false;
                        for disk in disks.list() {
                            if let Ok(disk_device_id) = std::fs::metadata(disk.mount_point()) {
                                if disk_device_id.st_dev() == data_directory_device_id {
                                    disk_available_and_total_space
                                        .replace((disk.available_space(), disk.total_space()));
                                    found = true;
                                    break;
                                }
                            }
                        }
                        if !found {
                            disk_available_and_total_space.take();
                        }
                    } else {
                        disk_available_and_total_space.take();
                    }
                }
                #[cfg(target_os = "windows")]
                {
                    let mut found = false;
                    for disk in disks.list() {
                        if data_directory.starts_with(disk.mount_point()) {
                            disk_available_and_total_space
                                .replace((disk.available_space(), disk.total_space()));
                            found = true;
                            break;
                        }
                    }
                    if !found {
                        disk_available_and_total_space.take();
                    }
                }
                #[cfg(all(
                    not(target_os = "linux"),
                    not(target_os = "macos"),
                    not(target_os = "windows")
                ))]
                {
                    let mut available_space = 0;
                    let mut total_space = 0;
                    for disk in disks.list() {
                        available_space += disk.available_space();
                        total_space += disk.total_space();
                    }
                    disk_available_and_total_space.replace((available_space, total_space));
                }
                if disk_available_and_total_space != previous_disk_available_and_total_space {
                    let mut context_guard = context.lock().await;
                    context_guard
                        .shared_client_state
                        .disk_available_and_total_space = disk_available_and_total_space;
                    if let Err(error) = context_guard.broadcast_shared_client_state() {
                        println!("broadcast_shared_client_state error: {error:?}");
                    }
                }
                tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            }
        });
    }

    // poll the TCP stream for new connections
    loop {
        if let Ok((stream, _)) = tcp_listener.accept().await {
            tokio::spawn(handle_tcp_stream(context.clone(), stream));
        }
    }
}

async fn handle_tcp_stream(
    context: std::sync::Arc<tokio::sync::Mutex<Context>>,
    stream: tokio::net::TcpStream,
) {
    let io = hyper_util::rt::TokioIo::new(stream);
    if let Err(error) = hyper::server::conn::http1::Builder::new()
        .serve_connection(
            io,
            hyper::service::service_fn(move |request| {
                handle_http_request(context.clone(), request)
            }),
        )
        .await
    {
        println!("{} | serve_connection error: {:?}", utc_string(), error);
    }
}

async fn handle_http_request(
    context: std::sync::Arc<tokio::sync::Mutex<Context>>,
    request: hyper::Request<hyper::body::Incoming>,
) -> Result<hyper::Response<http_body_util::Full<hyper::body::Bytes>>, anyhow::Error> {
    Ok(match request.uri().path() {
        "/transport-certificate" => match request.headers().get(hyper::header::HOST) {
            Some(host) => match host.to_str() {
                Ok(host) => {
                    let host = match host.rfind(':') {
                        Some(position) => {
                            if host
                                .as_bytes()
                                .iter()
                                .skip(position + 1)
                                .all(|character| character.is_ascii_digit())
                            {
                                &host[0..position]
                            } else {
                                host
                            }
                        }
                        None => host,
                    };
                    let (hash, port) = {
                        let mut context_guard = context.lock().await;
                        let (remove_host, reuse_port) =
                            match context_guard.host_to_endpoint.get(host) {
                                Some(endpoint) => {
                                    if endpoint.created.elapsed().is_ok_and(|elapsed| {
                                        elapsed > std::time::Duration::from_secs(7 * 24 * 60 * 60)
                                    }) {
                                        endpoint.terminate.notify_one();
                                        endpoint.terminated.notified().await;
                                        (true, Some(endpoint.port))
                                    } else {
                                        (false, None)
                                    }
                                }
                                None => (false, None),
                            };
                        if remove_host {
                            let _ = context_guard.host_to_endpoint.remove(host);
                        }
                        match context_guard.host_to_endpoint.get(host) {
                            Some(endpoint) => (endpoint.hash.clone(), endpoint.port),
                            None => {
                                let identity = wtransport::Identity::self_signed(&[host])?;
                                let endpoint = Endpoint::new(
                                    match reuse_port {
                                        Some(reuse_port) => reuse_port,
                                        None => {
                                            let new_port = context_guard.next_transport_port;
                                            context_guard.next_transport_port += 1;
                                            new_port
                                        }
                                    },
                                    &identity,
                                )?;
                                tokio::spawn(handle_transport_server(
                                    context.clone(),
                                    endpoint.terminate.clone(),
                                    endpoint.terminated.clone(),
                                    wtransport::Endpoint::server(
                                        wtransport::ServerConfig::builder()
                                            .with_bind_default(endpoint.port)
                                            .with_identity(identity)
                                            .build(),
                                    )?,
                                ));
                                println!(
                                    "{} | Listening for Transport requests on {}:{}",
                                    utc_string(),
                                    host,
                                    endpoint.port
                                );
                                let (hash, port) = (endpoint.hash.clone(), endpoint.port);
                                let _ = context_guard
                                    .host_to_endpoint
                                    .insert(host.to_owned(), endpoint);
                                (hash, port)
                            }
                        }
                    };
                    hyper::Response::builder()
                        .header(hyper::header::CONTENT_TYPE, "application/json")
                        .header(hyper::header::ACCESS_CONTROL_ALLOW_ORIGIN, "*")
                        .body(http_body_util::Full::new(hyper::body::Bytes::from(
                            format!("{{\"hash\":\"{}\",\"port\":{}}}", hash, port),
                        )))?
                }
                Err(host) => hyper::Response::builder()
                    .status(hyper::StatusCode::BAD_REQUEST)
                    .body(http_body_util::Full::new(hyper::body::Bytes::from(
                        format!("The request Host header contains non-ASCII characters ({host:?})"),
                    )))?,
            },
            None => hyper::Response::builder()
                .status(hyper::StatusCode::BAD_REQUEST)
                .body(http_body_util::Full::new(hyper::body::Bytes::from(
                    "The request has no Host header",
                )))?,
        },
        "/favicon.png" => hyper::Response::builder()
            .header(hyper::header::CONTENT_TYPE, "image/png")
            .body(http_body_util::Full::new(hyper::body::Bytes::from(
                include_bytes!("../ui/favicon.png").as_slice(),
            )))?,
        "/" => hyper::Response::builder()
            .header(hyper::header::CONTENT_TYPE, "text/html")
            .body(http_body_util::Full::new(hyper::body::Bytes::from(
                include_bytes!("../ui/build/index.html").as_slice(), // @DEV: feature flag to enable / disable embedding index.html
            )))?,
        _ => hyper::Response::builder()
            .status(hyper::StatusCode::NOT_FOUND)
            .body(http_body_util::Full::new(hyper::body::Bytes::from(
                "Not found",
            )))?,
    })
}

async fn handle_transport_server(
    context: std::sync::Arc<tokio::sync::Mutex<Context>>,
    terminate: std::sync::Arc<tokio::sync::Notify>,
    terminated: std::sync::Arc<tokio::sync::Notify>,
    transport_server: wtransport::endpoint::Endpoint<wtransport::endpoint::endpoint_side::Server>,
) {
    let mut incoming_session_id = 0;
    loop {
        tokio::select! {
            _ = terminate.notified() => {
                break;
            }
            incoming_session = transport_server.accept() => {
                tokio::spawn(handle_transport_session_wrapper(context.clone(), incoming_session, incoming_session_id));
                incoming_session_id += 1;
            }
        }
    }
    println!("destroy transport"); // @DEV
    drop(transport_server);
    terminated.notify_one();
}

async fn handle_transport_session_wrapper(
    context: std::sync::Arc<tokio::sync::Mutex<Context>>,
    incoming_session: wtransport::endpoint::IncomingSession,
    incoming_session_id: usize,
) {
    if let Err(error) =
        handle_transport_session(context.clone(), incoming_session, incoming_session_id).await
    {
        println!(
            "{} | handle_transport_session (session id {}) error: {:?}",
            utc_string(),
            incoming_session_id,
            error
        );
    }
}

async fn handle_transport_session(
    context: std::sync::Arc<tokio::sync::Mutex<Context>>,
    incoming_session: wtransport::endpoint::IncomingSession,
    incoming_session_id: usize,
) -> Result<(), anyhow::Error> {
    let session_request = incoming_session.await?;
    println!(
        "{} | new session (session id {}): authority {}, path {}",
        utc_string(),
        incoming_session_id,
        session_request.authority(),
        session_request.path()
    );
    let connection = session_request.accept().await?;
    let (client_proxy, shared_client_state_receiver) = client::ClientProxy::new();
    let client_id = {
        let mut context_guard = context.lock().await;
        let client_id = context_guard.next_client_id;
        context_guard.next_client_id.increment();
        context_guard.id_to_client.insert(client_id, client_proxy);
        client_id
    };
    let result = client::manage_connection(
        client_id,
        context.clone(),
        shared_client_state_receiver,
        connection,
        incoming_session_id,
    )
    .await;

    println!(
        "{} | manage_connection (session id {}) returned {:?}",
        utc_string(),
        incoming_session_id,
        result
    ); // @DEV

    {
        let mut context_guard = context.lock().await;
        let _ = context_guard.id_to_client.remove(&client_id);
    }
    result
}
