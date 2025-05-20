mod client;
mod constants;
mod device;
mod protocol;
mod stack;

use clap::Parser;

/*
use tokio::task::spawn_blocking;
use tokio::runtime::Handle;

async fn example() {
    let handle = Handle::current();
    spawn_blocking(move || {
        // do something blocking

        handle.block_on(async {
            // do something async
        });

        // do something blocking

        // ...
    });
}
*/

#[derive(clap::Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(short = 'p', long, default_value_t = 3000)]
    http_port: u16,

    #[arg(short = 'q', long, default_value_t = 3001)]
    transport_port: u16,

    #[arg(short = 'n', long, default_value_t = -1)]
    maximum_client_count: i64,

    #[arg(short = 'c', long, default_value_t = 60)]
    maximum_client_buffer_count: usize,

    #[arg(short = 's', long, default_value_t = 1usize << 30)]
    maximum_clients_buffering_memory: usize,
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
    maximum_client_count: Option<usize>,
    maximum_client_buffer_count: usize,
    client_count: usize,
    next_client_id: client::ClientId,
    id_to_client: std::collections::HashMap<client::ClientId, client::ClientProxy>,
    shared_client_state: protocol::SharedClientState,
    bus_number_and_address_to_device: std::collections::HashMap<(u8, u8), device::DeviceProxy>,
    router: std::sync::Arc<std::sync::RwLock<Router>>,
    packet_stack: std::sync::Arc<std::sync::Mutex<stack::Stack>>,
    sample_stack: std::sync::Arc<std::sync::Mutex<stack::Stack>>,
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
            .bus_number_and_address_to_device
            .iter()
            .map(|((bus_number, address), device)| protocol::Device {
                id: device.id.0,
                name: device.inner.name().to_owned(),
                serial: device.serial.clone(),
                speed: device.speed.to_string(),
                bus_number: *bus_number,
                address: *address,
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
            })
            .collect();
        std::mem::swap(&mut self.shared_client_state.devices, &mut devices);
        if let Err(error) = self.broadcast_shared_client_state() {
            println!("{error}");
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let time_reference = std::time::Instant::now();
    let args = Args::parse();
    println!("Listening for HTTP requests on port {}", args.http_port);
    let tcp_listener =
        tokio::net::TcpListener::bind((std::net::Ipv4Addr::new(0, 0, 0, 0), args.http_port))
            .await?;
    let context = std::sync::Arc::new(tokio::sync::Mutex::new(Context {
        time_reference,
        host_to_endpoint: std::collections::HashMap::new(),
        next_transport_port: args.transport_port,
        maximum_client_count: if args.maximum_client_count >= 0 {
            Some(args.maximum_client_count as usize)
        } else {
            None
        },
        maximum_client_buffer_count: args.maximum_client_buffer_count,
        client_count: 0,
        next_client_id: client::ClientId(0),
        id_to_client: std::collections::HashMap::new(),
        shared_client_state: protocol::SharedClientState {
            devices: Vec::new(),
        },
        bus_number_and_address_to_device: std::collections::HashMap::new(),
        router: std::sync::Arc::new(std::sync::RwLock::new(std::collections::HashMap::new())),
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
                                    && !context_guard
                                        .bus_number_and_address_to_device
                                        .contains_key(&(device.address, device.bus_number))
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
                                    println!("new device, id {}", next_device_id); // @DEV
                                    devices_and_proxies.push(device::Device::new(
                                        device::DeviceId(next_device_id),
                                        listed_device,
                                        device,
                                        flag,
                                    ));
                                    next_device_id = (next_device_id + 1) % 0x1000000;
                                }
                            }
                        }
                        let mut context_guard = context.blocking_lock();
                        for (bus_number_and_address, device, device_sampler, device_proxy) in
                            devices_and_proxies
                        {
                            {
                                let mut router_guard = context_guard
                                    .router
                                    .write()
                                    .expect("router mutex is not poisoned");
                                router_guard
                                    .insert(device::StreamId::new(device_proxy.id, 0), Vec::new());
                                router_guard
                                    .insert(device::StreamId::new(device_proxy.id, 1), Vec::new());
                                println!("{:?}", &router_guard); // @DEV
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
                                .bus_number_and_address_to_device
                                .insert(bus_number_and_address, device_proxy);
                        }
                        context_guard.update_shared_client_state_devices();
                    }
                }
                std::thread::sleep(std::time::Duration::from_secs(3));
            }
        });
    }

    // periodically check whether the (buffer) stack can be shrunk
    {
        let (packet_stack, sample_stack) = {
            let context_guard = context.lock().await;
            (
                context_guard.packet_stack.clone(),
                context_guard.sample_stack.clone(),
            )
        };
        tokio::spawn(async move {
            loop {
                packet_stack
                    .lock()
                    .expect("packet stack mutex is not poisoned")
                    .shrink_unused();

                /* @DEV
                sample_stack
                    .lock()
                    .expect("sample stack mutex is not poisoned")
                    .shrink_unused();
                */
                tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            }
        });
    }

    // poll the TCP stream for new connections
    loop {
        println!("poll TCP stream"); // @DEV
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
        println!("{error:?}");
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
                                    "Listening for Transport requests on {}:{}",
                                    host, endpoint.port
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
    loop {
        println!("poll transport session"); // @DEV
        tokio::select! {
            _ = terminate.notified() => {
                break;
            }
            incoming_session = transport_server.accept() => {
                tokio::spawn(handle_transport_session_wrapper(context.clone(), incoming_session));
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
) {
    {
        let mut context_guard = context.lock().await;
        if let Some(maximum_client_count) = context_guard.maximum_client_count {
            println!("maximum client count reached ({maximum_client_count})");
            // @DEV semd error to incoming_session
            return;
        }
        context_guard.client_count += 1;
    }
    if let Err(error) = handle_transport_session(context.clone(), incoming_session).await {
        println!("{error:?}");
    }
    {
        let mut context_guard = context.lock().await;
        if let Some(client_count) = context_guard.client_count.checked_sub(1) {
            context_guard.client_count = client_count;
        }
    }
}

async fn handle_transport_session(
    context: std::sync::Arc<tokio::sync::Mutex<Context>>,
    incoming_session: wtransport::endpoint::IncomingSession,
) -> Result<(), anyhow::Error> {
    let session_request = incoming_session.await?;
    println!(
        "New session: Authority: '{}', Path: '{}'",
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
    )
    .await;
    {
        let mut context_guard = context.lock().await;
        let _ = context_guard.id_to_client.remove(&client_id);
    }
    result
}
