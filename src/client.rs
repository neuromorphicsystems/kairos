use crate::constants;
use crate::device;
use crate::protocol;
use crate::Context;

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

pub async fn manage_connection(
    client_id: ClientId,
    context: std::sync::Arc<tokio::sync::Mutex<Context>>,
    mut shared_client_state_receiver: tokio::sync::mpsc::UnboundedReceiver<Vec<u8>>,
    connection: wtransport::Connection,
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
    let (router, packet_stack, sample_stack, maximum_client_buffer_count) = {
        let (
            shared_client_state_bytes,
            router,
            packet_stack,
            sample_stack,
            maximum_client_buffer_count,
        ) = {
            let context_guard = context.lock().await;
            (
                context_guard.shared_client_state.to_bytes()?,
                context_guard.router.clone(),
                context_guard.packet_stack.clone(),
                context_guard.sample_stack.clone(),
                context_guard.maximum_client_buffer_count,
            )
        };
        message_bidirectional_stream
            .0
            .write_all(&shared_client_state_bytes)
            .await?;
        (
            router,
            packet_stack,
            sample_stack,
            maximum_client_buffer_count,
        )
    };
    let mut message_buffer = vec![0; constants::MESSAGE_MAXIMUM_LENGTH as usize];
    let mut message_offset = 0;
    let mut message_length = 0;
    let result;
    let mut stream_handles = Vec::new();
    loop {
        tokio::select! {
            client_message_result = message_bidirectional_stream.1.read(&mut message_buffer[message_offset..]) => {
                match client_message_result {
                    Ok(bytes_read) => {
                        if let Some(bytes_read) = bytes_read {
                            message_offset += bytes_read;
                            loop {
                                if message_offset < 4 {
                                    break;
                                }
                                if message_length == 0 {
                                    message_length = u32::from_le_bytes(message_buffer[0..4].try_into().expect("4 bytes")) as usize;
                                }
                                if message_offset < message_length {
                                    break;
                                }
                                match serde_json::from_slice::<protocol::ClientMessage>(&message_buffer[4..message_length]) {
                                    Ok(message) => {
                                        println!("client message: {:?}", message); // @DEV
                                        match message {
                                            protocol::ClientMessage::StartStream { id } => {
                                                let stream_id = device::StreamId(id);
                                                let mut unidirectional_stream = connection.open_uni().await?.await?;
                                                unidirectional_stream.write_all(&protocol::stream_description(
                                                    stream_id.0,
                                                    constants::PACKET_RECOMMENDED_BUFFER_COUNT,
                                                    constants::PACKET_MAXIMUM_LENGTH,
                                                ))
                                                .await?;
                                                let (packet_sender, mut packet_receiver) = tokio::sync::mpsc::channel(maximum_client_buffer_count);
                                                let spawn = {
                                                    let mut router_guard = router.write().expect("router mutex is not poisoned");
                                                    if let Some(clients_ids_and_senders) = router_guard.get_mut(&stream_id) {
                                                        if clients_ids_and_senders.iter().any(|(sender_client_id, _)| *sender_client_id == client_id) {
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
                                                    let stack = if id.to_le_bytes()[3] == 0 {
                                                        packet_stack.clone()
                                                    } else {
                                                        sample_stack.clone()
                                                    };
                                                    let router = router.clone();
                                                    stream_handles.push(tokio::spawn(async move {
                                                        loop {
                                                            match packet_receiver.recv().await {
                                                                Some(packet) => {
                                                                    let result = unidirectional_stream.write_all(&packet).await;
                                                                    stack.lock().expect("stack mutex is not poisoned").push(packet);
                                                                    if result.is_err() {
                                                                        break;
                                                                    }
                                                                },
                                                                None => break,
                                                            }
                                                        }
                                                        {
                                                            let mut router_guard = router.write().expect("router mutex is not poisoned");
                                                            if let Some(clients_ids_and_senders) = router_guard.get_mut(&stream_id) {
                                                                let mut removed = 0;
                                                                for index in 0..clients_ids_and_senders.len() {
                                                                    if clients_ids_and_senders[index - removed].0 == client_id {
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
                                                                        "removed {} entries for client {} and stream {} in router during clearnup",
                                                                        removed,
                                                                        client_id.0,
                                                                        stream_id.0
                                                                    );
                                                                }
                                                            } else {
                                                                println!("client {} not found in router during cleanup", client_id.0);
                                                            }
                                                        }
                                                        loop {
                                                            match packet_receiver.recv().await {
                                                                Some(packet) => {
                                                                    stack
                                                                        .lock()
                                                                        .expect("stack mutex is not poisoned")
                                                                        .push(packet);
                                                                }
                                                                None => break,
                                                            }
                                                        }
                                                    }));
                                                }
                                            },
                                            protocol::ClientMessage::RecordTarget { id, lookback, autotrigger, path } => {

                                            },
                                        }
                                    }
                                    Err(error) => {
                                        println!("parsing the message '{}' failed ({:?})", String::from_utf8_lossy(&message_buffer[4..message_length]), error);
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
                        break;
                    }
                }
            }
            shared_client_state = shared_client_state_receiver.recv() => {
                if let Some(shared_client_state_bytes) = shared_client_state {
                    message_bidirectional_stream
                        .0
                        .write_all(&shared_client_state_bytes)
                        .await?;
                }
            }
        }
    }
    {
        let mut router_guard = router.write().expect("router mutex is not poisoned");
        for (_, clients_ids_and_senders) in router_guard.iter_mut() {
            let mut removed = 0;
            for index in 0..clients_ids_and_senders.len() {
                if clients_ids_and_senders[index - removed].0 == client_id {
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
            println!("{error:?}");
        }
    }
    result
}
