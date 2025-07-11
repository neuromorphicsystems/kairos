use crate::constants;
use crate::device;
use crate::now_utc_string;
use crate::protocol;
use crate::stack;
use anyhow::anyhow;

use neuromorphic_drivers::UsbDevice;

pub struct ClientProxy {
    pub shared_client_state_sender: tokio::sync::mpsc::UnboundedSender<Vec<u8>>,
}

impl ClientProxy {
    pub fn new() -> (Self, tokio::sync::mpsc::UnboundedReceiver<Vec<u8>>) {
        let (shared_client_state_sender, shared_client_state_receiver) =
            tokio::sync::mpsc::unbounded_channel();
        (
            Self {
                shared_client_state_sender,
            },
            shared_client_state_receiver,
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ClientId(pub u32);

impl ClientId {
    pub fn increment(&mut self) {
        self.0 = self.0.wrapping_add(1);
    }
}

fn spawn_stream(
    stream_id: device::StreamId,
    client_id: ClientId,
    incoming_session_id: usize,
    mut packet_receiver: tokio::sync::mpsc::Receiver<Vec<u8>>,
    mut unidirectional_stream: wtransport::SendStream,
    stack: std::sync::Arc<std::sync::Mutex<stack::Stack>>,
    router: std::sync::Arc<std::sync::RwLock<crate::Router>>,
    only_send_if_different: bool,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let mut last_sent_packet: Option<Vec<u8>> = None;
        loop {
            match packet_receiver.recv().await {
                Some(packet) => {
                    let result = if only_send_if_different {
                        match last_sent_packet.as_mut() {
                            Some(last_sent_packet) => {
                                if *last_sent_packet == packet {
                                    Ok(())
                                } else {
                                    last_sent_packet.clear();
                                    last_sent_packet.resize(packet.len(), 0u8);
                                    last_sent_packet.copy_from_slice(&packet);
                                    unidirectional_stream.write_all(&packet).await
                                }
                            }
                            None => {
                                last_sent_packet.replace(packet.clone());
                                unidirectional_stream.write_all(&packet).await
                            }
                        }
                    } else {
                        unidirectional_stream.write_all(&packet).await
                    };
                    stack.lock().expect("stack mutex is poisoned").push(packet);
                    if result.is_err() {
                        break;
                    }
                }
                None => {
                    break;
                }
            }
        }
        {
            let mut router_guard = router.write().expect("router mutex is poisoned");
            if let Some(clients_ids_and_senders) = router_guard.get_mut(&stream_id) {
                let mut removed = 0;
                for index in 0..clients_ids_and_senders.len() {
                    if clients_ids_and_senders[index].0 == client_id {
                        removed += 1;
                    } else {
                        clients_ids_and_senders.swap(index - removed, index);
                    }
                }
                for _ in 0..removed {
                    let _ = clients_ids_and_senders.pop();
                }
                if removed != 1 {
                    println!(
                        "{} | removed {} entries for client {} and stream {} in router during cleanup (session id {})",
                        now_utc_string(),
                        removed,
                        client_id.0,
                        stream_id.0,
                        incoming_session_id,
                    );
                }
            } else {
                println!("client {} not found in router during cleanup", client_id.0);
            }
        }
        loop {
            match packet_receiver.recv().await {
                Some(packet) => {
                    stack.lock().expect("stack mutex is poisoned").push(packet);
                }
                None => break,
            }
        }
    })
}

async fn handle_client_message(
    client_id: ClientId,
    incoming_session_id: usize,
    message: protocol::ClientMessage,
    message_bidirectional_stream: &mut (wtransport::SendStream, wtransport::RecvStream),
    connection: &wtransport::Connection,
    pong_bytes: &[u8],
    maximum_client_buffer_count: usize,
    context: &std::sync::Arc<tokio::sync::Mutex<crate::Context>>,
    router: &std::sync::Arc<std::sync::RwLock<crate::Router>>,
    packet_stack: &std::sync::Arc<std::sync::Mutex<stack::Stack>>,
    sample_stack: &std::sync::Arc<std::sync::Mutex<stack::Stack>>,
    stream_handles: &mut Vec<tokio::task::JoinHandle<()>>,
) -> Result<(), anyhow::Error> {
    match message {
        protocol::ClientMessage::Ping => {
            if let Err(error) = message_bidirectional_stream.0.write_all(pong_bytes).await {
                Err(error.into())
            } else {
                Ok(())
            }
        }
        protocol::ClientMessage::StartStream { stream_id } => {
            let stream_id = device::StreamId(stream_id);
            let mut unidirectional_stream = connection.open_uni().await?.await?;
            unidirectional_stream
                .write_all(&protocol::stream_description(
                    stream_id.0,
                    if stream_id.stream_index() == 0 {
                        constants::PACKET_RECOMMENDED_BUFFER_COUNT
                    } else {
                        constants::SAMPLE_RECOMMENDED_BUFFER_COUNT
                    },
                    if stream_id.stream_index() == 0 {
                        constants::PACKET_MAXIMUM_LENGTH
                    } else {
                        constants::SAMPLE_MAXIMUM_LENGTH
                    },
                ))
                .await?;
            let (packet_sender, packet_receiver) =
                tokio::sync::mpsc::channel(maximum_client_buffer_count);
            let spawn = {
                let mut router_guard = router.write().expect("router mutex is poisoned");
                if let Some(clients_ids_and_senders) = router_guard.get_mut(&stream_id) {
                    if clients_ids_and_senders
                        .iter()
                        .any(|(sender_client_id, _)| *sender_client_id == client_id)
                    {
                        false
                    } else {
                        clients_ids_and_senders.push((client_id, packet_sender));
                        true
                    }
                } else {
                    false
                }
            };
            if spawn {
                stream_handles.push(spawn_stream(
                    stream_id,
                    client_id,
                    incoming_session_id,
                    packet_receiver,
                    unidirectional_stream,
                    if stream_id.stream_index() == 0 {
                        packet_stack.clone()
                    } else {
                        sample_stack.clone()
                    },
                    router.clone(),
                    false,
                ));
            }
            Ok(())
        }
        protocol::ClientMessage::UpdateConfiguration {
            device_id,
            configuration,
        } => {
            let device_id = device::DeviceId(device_id);
            let mut context_guard = context.lock().await;
            for device in context_guard.shared_client_state.devices.iter_mut() {
                if device.id == device_id.0 {
                    device.configuration = configuration.clone();
                    break;
                }
            }
            if let Some(device_proxy) = context_guard.id_to_device.get_mut(&device_id) {
                match device_proxy.inner.as_ref() {
                    neuromorphic_drivers::Device::InivationDavis346(device) => {
                        match configuration {
                            neuromorphic_drivers::Configuration::InivationDavis346(
                                configuration,
                            ) => {
                                device.update_configuration(configuration);
                                device_proxy
                                    .configuration_changed
                                    .store(true, std::sync::atomic::Ordering::Release);
                            }
                            _ => println!(
                                "mismatch between the configuration type and the device type"
                            ),
                        }
                    }
                    neuromorphic_drivers::Device::PropheseeEvk3Hd(device) => match configuration {
                        neuromorphic_drivers::Configuration::PropheseeEvk3Hd(configuration) => {
                            device.update_configuration(configuration);
                            device_proxy
                                .configuration_changed
                                .store(true, std::sync::atomic::Ordering::Release);
                        }
                        _ => {
                            println!("mismatch between the configuration type and the device type")
                        }
                    },
                    neuromorphic_drivers::Device::PropheseeEvk4(device) => match configuration {
                        neuromorphic_drivers::Configuration::PropheseeEvk4(configuration) => {
                            device.update_configuration(configuration);
                            device_proxy
                                .configuration_changed
                                .store(true, std::sync::atomic::Ordering::Release);
                        }
                        _ => {
                            println!("mismatch between the configuration type and the device type")
                        }
                    },
                }
            } else {
                println!("unknown device id {} in SetParameter message", device_id.0);
            }
            if let Err(error) = context_guard.broadcast_shared_client_state() {
                println!("broadcast_shared_client_state error: {error:?}");
            }
            Ok(())
        }
        protocol::ClientMessage::UpdateLookback {
            device_id,
            lookback,
        } => {
            let device_id = device::DeviceId(device_id);
            let mut context_guard = context.lock().await;
            if let Some(device) = context_guard.id_to_device.get_mut(&device_id) {
                let mut record_configuration_guard = device
                    .record_configuration
                    .lock()
                    .expect("record configuration mutex is poisoned");
                record_configuration_guard.lookback = lookback;
            } else {
                println!(
                    "unknown device id {} in UpdateLookback message",
                    device_id.0
                );
            }
            for context_device in context_guard.shared_client_state.devices.iter_mut() {
                if context_device.id == device_id.0 {
                    context_device.lookback = lookback;
                    if let Err(error) = context_guard.broadcast_shared_client_state() {
                        println!("broadcast_shared_client_state error: {error:?}");
                    }
                    break;
                }
            }
            Ok(())
        }
        protocol::ClientMessage::UpdateAutostop {
            device_id,
            autostop,
        } => {
            let device_id = device::DeviceId(device_id);
            let mut context_guard = context.lock().await;
            if let Some(device) = context_guard.id_to_device.get_mut(&device_id) {
                let mut record_configuration_guard = device
                    .record_configuration
                    .lock()
                    .expect("record configuration mutex is poisoned");
                record_configuration_guard.autostop = autostop;
            } else {
                println!(
                    "unknown device id {} in UpdateAutostop message",
                    device_id.0
                );
            }
            for context_device in context_guard.shared_client_state.devices.iter_mut() {
                if context_device.id == device_id.0 {
                    context_device.autostop = autostop;
                    if let Err(error) = context_guard.broadcast_shared_client_state() {
                        println!("broadcast_shared_client_state error: {error:?}");
                    }
                    break;
                }
            }
            Ok(())
        }
        protocol::ClientMessage::UpdateAutotrigger {
            device_id,
            autotrigger,
        } => {
            let device_id = device::DeviceId(device_id);
            let mut context_guard = context.lock().await;
            if let Some(device) = context_guard.id_to_device.get_mut(&device_id) {
                let mut record_configuration_guard = device
                    .record_configuration
                    .lock()
                    .expect("record configuration mutex is poisoned");
                record_configuration_guard.autotrigger = autotrigger.clone();
            } else {
                println!(
                    "unknown device id {} in UpdateAutotrigger message",
                    device_id.0
                );
            }
            for context_device in context_guard.shared_client_state.devices.iter_mut() {
                if context_device.id == device_id.0 {
                    context_device.autotrigger = autotrigger.clone();
                    if let Err(error) = context_guard.broadcast_shared_client_state() {
                        println!("broadcast_shared_client_state error: {error:?}");
                    }
                    break;
                }
            }
            Ok(())
        }
        protocol::ClientMessage::StartRecording { device_id, name } => {
            let device_id = device::DeviceId(device_id);
            let mut context_guard = context.lock().await;
            if let Some(device) = context_guard.id_to_device.get_mut(&device_id) {
                let mut record_configuration = device
                    .record_configuration
                    .lock()
                    .expect("record configuration mutex is poisoned");
                record_configuration.action = device::RecordAction::Start(name);
            } else {
                println!(
                    "unknown device id {} in StartRecording message",
                    device_id.0
                );
            }
            Ok(())
        }
        protocol::ClientMessage::StopRecording { device_id } => {
            let device_id = device::DeviceId(device_id);
            let context_guard = context.lock().await;
            if let Some(device) = context_guard.id_to_device.get(&device_id) {
                let mut record_configuration = device
                    .record_configuration
                    .lock()
                    .expect("record configuration mutex is poisoned");
                record_configuration.action = device::RecordAction::Stop;
            } else {
                println!(
                    "unknown device id {} in StartRecording message",
                    device_id.0
                );
            }
            Ok(())
        }
        protocol::ClientMessage::Convert { mut names } => {
            let mut context_guard = context.lock().await;
            names.sort();
            let mut name_index = 0;
            let mut recording_index = 0;
            while name_index < names.len()
                && recording_index < context_guard.shared_recordings_state.recordings.len()
            {
                match names[name_index]
                    .cmp(&context_guard.shared_recordings_state.recordings[recording_index].name)
                {
                    std::cmp::Ordering::Less => {
                        name_index += 1;
                    }
                    std::cmp::Ordering::Equal => {
                        if let protocol::RecordingState::Complete { size_bytes, zip } =
                            context_guard.shared_recordings_state.recordings[recording_index].state
                        {
                            if !zip {
                                context_guard.shared_recordings_state.recordings[recording_index]
                                    .state = protocol::RecordingState::Queued { size_bytes, zip };
                            }
                        }
                        name_index += 1;
                        recording_index += 1;
                    }
                    std::cmp::Ordering::Greater => {
                        recording_index += 1;
                    }
                }
            }
            context_guard.notify_convert.notify_one();
            Ok(())
        }
        protocol::ClientMessage::CancelConvert => {
            let mut context_guard = context.lock().await;
            let mut changed = false;
            for recording in context_guard.shared_recordings_state.recordings.iter_mut() {
                if let protocol::RecordingState::Queued { size_bytes, zip } = recording.state {
                    recording.state = protocol::RecordingState::Complete { size_bytes, zip };
                    changed = true;
                }
            }
            if changed {
                if let Err(error) = context_guard.broadcast_shared_recordings_state() {
                    println!("broadcast_recordings error: {error:?}");
                }
            }
            context_guard.notify_convert_cancel.notify_one();
            Ok(())
        }
    }
}

pub async fn manage_connection(
    client_id: ClientId,
    context: std::sync::Arc<tokio::sync::Mutex<crate::Context>>,
    mut shared_client_state_receiver: tokio::sync::mpsc::UnboundedReceiver<Vec<u8>>,
    connection: wtransport::Connection,
    incoming_session_id: usize,
) -> Result<(), anyhow::Error> {
    let mut message_bidirectional_stream = connection.open_bi().await?.await?;
    message_bidirectional_stream
        .0
        .write_all(&protocol::stream_description(
            constants::MESSAGE_STREAM_ID,
            constants::MESSAGE_RECOMMENDED_BUFFER_COUNT,
            constants::MESSAGE_MAXIMUM_LENGTH,
        ))
        .await?;
    let mut record_state_unidirectional_stream = connection.open_uni().await?.await?;
    record_state_unidirectional_stream
        .write_all(&protocol::stream_description(
            constants::RECORD_STATE_STREAM_ID,
            constants::RECORD_STATE_RECOMMENDED_BUFFER_COUNT,
            constants::RECORD_STATE_MAXIMUM_LENGTH,
        ))
        .await?;
    let pong_bytes = [8, 0, 0, 0, b'p', b'o', b'n', b'g'];
    let (router, packet_stack, sample_stack, record_state_stack, maximum_client_buffer_count) = {
        let (
            shared_client_state_bytes,
            recordings_bytes,
            router,
            packet_stack,
            sample_stack,
            record_state_stack,
            maximum_client_buffer_count,
        ) = {
            let context_guard = context.lock().await;
            (
                protocol::ServerMessage::SharedClientState(&context_guard.shared_client_state)
                    .to_bytes()?,
                protocol::ServerMessage::SharedRecordingsState(
                    &context_guard.shared_recordings_state,
                )
                .to_bytes()?,
                context_guard.router.clone(),
                context_guard.packet_stack.clone(),
                context_guard.sample_stack.clone(),
                context_guard.record_state_stack.clone(),
                context_guard.maximum_client_buffer_count,
            )
        };
        message_bidirectional_stream
            .0
            .write_all(&shared_client_state_bytes)
            .await?;
        message_bidirectional_stream
            .0
            .write_all(&recordings_bytes)
            .await?;
        (
            router,
            packet_stack,
            sample_stack,
            record_state_stack,
            maximum_client_buffer_count,
        )
    };
    let (record_state_sender, record_state_receiver) =
        tokio::sync::mpsc::channel(maximum_client_buffer_count);
    {
        let mut router_guard = router.write().expect("router mutex is poisoned");
        if let Some(clients_ids_and_senders) =
            router_guard.get_mut(&device::StreamId(constants::RECORD_STATE_STREAM_ID))
        {
            clients_ids_and_senders.push((client_id, record_state_sender));
        }
    }
    let mut message_buffer = vec![0; constants::MESSAGE_MAXIMUM_LENGTH as usize];
    let mut message_offset = 0;
    let mut message_length = 0;
    let result;
    let mut stream_handles = Vec::new();
    stream_handles.push(spawn_stream(
        device::StreamId(constants::RECORD_STATE_STREAM_ID),
        client_id,
        incoming_session_id,
        record_state_receiver,
        record_state_unidirectional_stream,
        record_state_stack,
        router.clone(),
        true,
    ));
    'client: loop {
        tokio::select! {
            client_message_result = message_bidirectional_stream.1.read(&mut message_buffer[message_offset..]) => {
                match client_message_result {
                    Ok(bytes_read) => {
                        if let Some(bytes_read) = bytes_read {
                            message_offset += bytes_read;
                            'message: loop {
                                if message_offset < 4 {
                                    break 'message;
                                }
                                if message_length == 0 {
                                    message_length = u32::from_le_bytes(message_buffer[0..4].try_into().expect("4 bytes")) as usize;
                                }
                                if message_offset < message_length {
                                    break 'message;
                                }
                                match serde_json::from_slice::<protocol::ClientMessage>(&message_buffer[4..message_length]) {
                                    Ok(message) => {
                                        if let Err(error) = handle_client_message(
                                            client_id,
                                            incoming_session_id,
                                            message,
                                            &mut message_bidirectional_stream,
                                            &connection,
                                            &pong_bytes,
                                            maximum_client_buffer_count,
                                            &context,
                                            &router,
                                            &packet_stack,
                                            &sample_stack,
                                            &mut stream_handles,
                                        ).await {
                                            result = Err(error.into());
                                            break 'client;
                                        }
                                    }
                                    Err(error) => {
                                        println!(
                                            "parsing the message '{}' failed ({:?})",
                                            String::from_utf8_lossy(&message_buffer[4..message_length]),
                                            error
                                        );
                                    }
                                }
                                message_buffer.as_mut_slice().copy_within(message_length..message_offset, 0);
                                message_offset -= message_length;
                                message_length = 0;
                            }
                        }
                    }
                    Err(error) => {
                        result = Err(error.into());
                        break 'client;
                    }
                }
            }
            shared_client_state = shared_client_state_receiver.recv() => {
                if let Some(shared_client_state_bytes) = shared_client_state {
                    if let Err(error) = message_bidirectional_stream
                        .0
                        .write_all(&shared_client_state_bytes)
                        .await {
                            result = Err(error.into());
                            break 'client;
                        }
                } else {
                    result = Err(anyhow!("shared_client_state_receiver closed"));
                    break 'client;
                }
            }
        }
    }
    {
        let mut router_guard = router.write().expect("router mutex is poisoned");
        for (_, clients_ids_and_senders) in router_guard.iter_mut() {
            let mut removed = 0;
            for index in 0..clients_ids_and_senders.len() {
                if clients_ids_and_senders[index].0 == client_id {
                    removed += 1;
                } else {
                    clients_ids_and_senders.swap(index - removed, index);
                }
            }
            for _ in 0..removed {
                let _ = clients_ids_and_senders.pop();
            }
        }
    }
    for stream_handle in stream_handles {
        if let Err(error) = stream_handle.await {
            println!(
                "{} | stream_handle error (session id {}): {:?}",
                now_utc_string(),
                incoming_session_id,
                error
            );
        }
    }
    result
}
