use crate::queue::queue_display::{build_boss_bar_remove_packet, build_display_packets};
use crate::queue::PushAction;
use crate::server::client_data::ClientData;
use crate::server::packet_handler::{PacketHandler, PacketHandlerError};
use crate::server::packet_registry::{
    PacketRegistry, PacketRegistryDecodeError, PacketRegistryEncodeError,
};
use crate::server::shutdown_signal::shutdown_signal;
use crate::server_state::ServerState;
use futures::StreamExt;
use minecraft_packets::login::login_disconnect_packet::LoginDisconnectPacket;
use minecraft_packets::play::client_bound_keep_alive_packet::ClientBoundKeepAlivePacket;
use minecraft_packets::play::disconnect_packet::DisconnectPacket;
use minecraft_packets::play::transfer_packet::TransferPacket;
use minecraft_protocol::prelude::{State, VarInt};
use net::packet_stream::PacketStreamError;
use net::raw_packet::RawPacket;
use std::num::TryFromIntError;
use std::sync::Arc;
use std::time::Duration;
use thiserror::Error;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::RwLock;
use tokio::sync::oneshot;
use tracing::{debug, error, info, trace, warn};

pub struct Server {
    state: Arc<RwLock<ServerState>>,
    listen_address: String,
}

impl Server {
    pub fn new(listen_address: &impl ToString, state: ServerState) -> Self {
        Self {
            state: Arc::new(RwLock::new(state)),
            listen_address: listen_address.to_string(),
        }
    }

    pub async fn run(self) {
        let listener = match TcpListener::bind(&self.listen_address).await {
            Ok(sock) => sock,
            Err(err) => {
                error!("Failed to bind to {}: {}", self.listen_address, err);
                std::process::exit(1);
            }
        };

        info!("Listening on: {}", self.listen_address);
        self.accept(&listener).await;
    }

    pub async fn accept(self, listener: &TcpListener) {
        loop {
            tokio::select! {
                 accept_result = listener.accept() => {
                    match accept_result {
                        Ok((socket, addr)) => {
                            debug!("Accepted connection from {}", addr);
                        let state_clone = Arc::clone(&self.state);
                            tokio::spawn(async move {
                                handle_client(socket, state_clone).await;
                            });
                        }
                        Err(e) => {
                            error!("Failed to accept a connection: {:?}", e);
                        }
                    }
                },

                 () = shutdown_signal() => {
                    info!("Shutdown signal received, shutting down gracefully.");
                    break;
                }
            }
        }
    }
}

#[derive(Debug, Error)]
pub enum PacketProcessingError {
    #[error("Client disconnected")]
    Disconnected,

    #[error("Packet not found version={0} state={1} packet_id={2}")]
    DecodePacketError(i32, State, u8),

    #[error("{0}")]
    Custom(String),
}

impl From<PacketHandlerError> for PacketProcessingError {
    fn from(e: PacketHandlerError) -> Self {
        match e {
            PacketHandlerError::Custom(reason) => Self::Custom(reason),
            PacketHandlerError::InvalidState(reason, should_warn) => {
                if should_warn {
                    warn!("{reason}");
                } else {
                    debug!("{reason}");
                }
                Self::Disconnected
            }
        }
    }
}

impl From<PacketRegistryDecodeError> for PacketProcessingError {
    fn from(e: PacketRegistryDecodeError) -> Self {
        match e {
            PacketRegistryDecodeError::NoCorrespondingPacket(version, state, packet_id) => {
                Self::DecodePacketError(version, state, packet_id)
            }
            _ => Self::Custom(e.to_string()),
        }
    }
}

impl From<PacketRegistryEncodeError> for PacketProcessingError {
    fn from(e: PacketRegistryEncodeError) -> Self {
        Self::Custom(e.to_string())
    }
}

impl From<TryFromIntError> for PacketProcessingError {
    fn from(e: TryFromIntError) -> Self {
        Self::Custom(e.to_string())
    }
}

impl From<PacketStreamError> for PacketProcessingError {
    fn from(value: PacketStreamError) -> Self {
        match value {
            PacketStreamError::Io(ref e)
                if e.kind() == std::io::ErrorKind::UnexpectedEof
                    || e.kind() == std::io::ErrorKind::ConnectionReset =>
            {
                Self::Disconnected
            }
            _ => Self::Custom(value.to_string()),
        }
    }
}

async fn process_packet(
    client_data: &ClientData,
    server_state: &Arc<RwLock<ServerState>>,
    raw_packet: RawPacket,
    was_in_play_state: &mut bool,
) -> Result<(), PacketProcessingError> {
    let mut client_state = client_data.client().await;
    let protocol_version = client_state.protocol_version();
    let state = client_state.state();
    let decoded_packet = PacketRegistry::decode_packet(protocol_version, state, raw_packet)?;

    let batch = {
        let server_state_guard = server_state.read().await;
        decoded_packet.handle(&mut client_state, &server_state_guard)?
    };

    let protocol_version = client_state.protocol_version();
    let state = client_state.state();

    if !*was_in_play_state && state == State::Play {
        *was_in_play_state = true;
        server_state.write().await.increment();
        let username = client_state.get_username();
        debug!(
            "{} joined using version {}",
            username,
            protocol_version.humanize()
        );
        info!("{} joined the game", username,);
    }

    let mut stream = batch.into_stream();
    while let Some(pending_packet) = stream.next().await {
        let enable_compression = matches!(pending_packet, PacketRegistry::SetCompression(..));
        let raw_packet = pending_packet.encode_packet(protocol_version)?;
        client_data.write_packet(raw_packet).await?;
        if enable_compression
            && let Some(compression_settings) = server_state.read().await.compression_settings()
        {
            let mut packet_stream = client_data.stream().await;
            packet_stream
                .set_compression(compression_settings.threshold, compression_settings.level);
        }
    }

    if let Some(reason) = client_state.should_kick() {
        drop(client_state);
        kick_client(client_data, reason.clone())
            .await
            .map_err(|_| PacketProcessingError::Disconnected)?;
        return Err(PacketProcessingError::Disconnected);
    }

    drop(client_state);
    client_data.enable_keep_alive_if_needed().await;

    Ok(())
}

async fn read(
    client_data: &ClientData,
    server_state: &Arc<RwLock<ServerState>>,
    was_in_play_state: &mut bool,
) -> Result<(), PacketProcessingError> {
    tokio::select! {
        result = client_data.read_packet() => {
            let raw_packet = result?;
            process_packet(client_data, server_state, raw_packet, was_in_play_state).await?;
        }
        () = client_data.keep_alive_tick() => {
            send_keep_alive(client_data).await?;
        }
    }
    Ok(())
}

async fn handle_client(socket: TcpStream, server_state: Arc<RwLock<ServerState>>) {
    let client_data = ClientData::new(socket);
    let mut was_in_play_state = false;
    // oneshot receiver for queue push signal; set when player enters queue
    let mut push_rx: Option<oneshot::Receiver<PushAction>> = None;
    // cancel token for the display refresh task
    let mut display_cancel_tx: Option<oneshot::Sender<()>> = None;

    loop {
        // If we have a push receiver, select on it alongside normal read
        let push_triggered = if let Some(rx) = push_rx.as_mut() {
            tokio::select! {
                read_result = read(&client_data, &server_state, &mut was_in_play_state) => {
                    match read_result {
                        Ok(()) => None,
                        Err(PacketProcessingError::Disconnected) => {
                            debug!("Client disconnected");
                            break;
                        }
                        Err(PacketProcessingError::Custom(e)) => {
                            debug!("Error processing packet: {}", e);
                            None
                        }
                        Err(PacketProcessingError::DecodePacketError(version, state, packet_id)) => {
                            trace!("Unknown packet received: version={version} state={state} packet_id={packet_id}");
                            None
                        }
                    }
                }
                push_result = rx => {
                    match push_result {
                        Ok(action) => Some(action),
                        Err(_) => None, // sender dropped
                    }
                }
            }
        } else {
            match read(&client_data, &server_state, &mut was_in_play_state).await {
                Ok(()) => None,
                Err(PacketProcessingError::Disconnected) => {
                    debug!("Client disconnected");
                    break;
                }
                Err(PacketProcessingError::Custom(e)) => {
                    debug!("Error processing packet: {}", e);
                    None
                }
                Err(PacketProcessingError::DecodePacketError(version, state, packet_id)) => {
                    trace!(
                        "Unknown packet received: version={version} state={state} packet_id={packet_id}"
                    );
                    None
                }
            }
        };

        // Check if we just entered Play state and need to enqueue
        if was_in_play_state && push_rx.is_none() {
            let queue_state = server_state.read().await.queue_state();
            if let Some(qs) = queue_state {
                let (uuid, username) = {
                    let cs = client_data.client().await;
                    (cs.get_unique_id(), cs.get_username())
                };
                let (tx, rx) = oneshot::channel::<PushAction>();
                push_rx = Some(rx);
                let _position = qs.enqueue(uuid, username, tx).await;

                // Spawn display refresh task
                let qs_display = Arc::clone(&qs);
                let client_data_display = client_data.clone();
                let server_state_display = Arc::clone(&server_state);
                let refresh_interval =
                    Duration::from_secs(qs.settings.refresh_interval_seconds);
                let (cancel_tx, mut cancel_rx) = oneshot::channel::<()>();
                display_cancel_tx = Some(cancel_tx);
                tokio::spawn(async move {
                    loop {
                        tokio::select! {
                            _ = tokio::time::sleep(refresh_interval) => {}
                            _ = &mut cancel_rx => { break; }
                        }

                        let position = qs_display.position_of(uuid).await;
                        let total = qs_display.total().await;

                        if let Some(position) = position {
                            let (protocol_version, boss_bar_uuid) = {
                                let cs = client_data_display.client().await;
                                (cs.protocol_version(), cs.boss_bar_uuid())
                            };
                            let queue_config = {
                                let ss = server_state_display.read().await;
                                ss.queue_config().cloned()
                            };
                            if let Some(ref cfg) = queue_config {
                                let username_str = qs_display
                                    .username_of(uuid)
                                    .await
                                    .unwrap_or_default();
                                let packets = build_display_packets(
                                    cfg,
                                    position,
                                    total,
                                    &username_str,
                                    protocol_version,
                                    boss_bar_uuid,
                                );
                                for raw in packets {
                                    if client_data_display.write_packet(raw).await.is_err() {
                                        return;
                                    }
                                }
                            }
                        }
                    }
                });
            }
        }

        // Handle push action
        if let Some(action) = push_triggered {
            // Cancel display task
            if let Some(tx) = display_cancel_tx.take() {
                let _ = tx.send(());
            }

            // Send boss bar remove
            {
                let queue_state = server_state.read().await.queue_state();
                if let Some(_qs) = queue_state {
                    let (protocol_version, boss_bar_uuid) = {
                        let cs = client_data.client().await;
                        (cs.protocol_version(), cs.boss_bar_uuid())
                    };
                    let queue_config = server_state.read().await.queue_config().cloned();
                    if let Some(ref cfg) = queue_config {
                        if let Some(raw) = build_boss_bar_remove_packet(cfg, boss_bar_uuid, protocol_version) {
                            let _ = client_data.write_packet(raw).await;
                        }
                    }
                }
            }

            // Dequeue from state
            {
                let queue_state = server_state.read().await.queue_state();
                if let Some(qs) = queue_state {
                    let uuid = client_data.client().await.get_unique_id();
                    qs.dequeue(uuid).await;
                }
            }

            match action {
                PushAction::Kick(msg) => {
                    let _ = kick_client(&client_data, msg).await;
                }
                PushAction::Transfer { host, port } => {
                    let (protocol_version, supports_transfer) = {
                        let cs = client_data.client().await;
                        let pv = cs.protocol_version();
                        let supports = pv.is_after_inclusive(
                            minecraft_protocol::prelude::ProtocolVersion::V1_20_5,
                        );
                        (pv, supports)
                    };
                    if supports_transfer {
                        let packet = TransferPacket::new(&host, &VarInt::from(port));
                        if let Ok(raw) =
                            PacketRegistry::Transfer(packet).encode_packet(protocol_version)
                        {
                            let _ = client_data.write_packet(raw).await;
                        }
                    } else {
                        let _ = kick_client(
                            &client_data,
                            "You have been moved to the main server.".to_string(),
                        )
                        .await;
                    }
                }
            }
            break;
        }
    }

    // Cancel display task on any exit
    if let Some(tx) = display_cancel_tx.take() {
        let _ = tx.send(());
    }

    // Dequeue on disconnect
    {
        let queue_state = server_state.read().await.queue_state();
        if let Some(qs) = queue_state {
            let uuid = client_data.client().await.get_unique_id();
            qs.dequeue(uuid).await;
        }
    }

    let _ = client_data.shutdown().await;

    if was_in_play_state {
        server_state.write().await.decrement();
        let username = client_data.client().await.get_username();
        info!("{} left the game", username);
    }
}

async fn kick_client(
    client_data: &ClientData,
    reason: String,
) -> Result<(), PacketProcessingError> {
    let (protocol_version, state) = {
        let state = client_data.client().await;
        (state.protocol_version(), state.state())
    };
    let packet = match state {
        State::Login => {
            debug!("Login disconnect");
            PacketRegistry::LoginDisconnect(LoginDisconnectPacket::text(reason))
        }
        State::Configuration => {
            debug!("Configuration disconnect");
            PacketRegistry::ConfigurationDisconnect(DisconnectPacket::text(reason))
        }
        State::Play => {
            debug!("Play disconnect");
            PacketRegistry::PlayDisconnect(DisconnectPacket::text(reason))
        }
        _ => {
            debug!("A user was disconnected from a state where no packet can be sent");
            return Err(PacketProcessingError::Disconnected);
        }
    };
    if let Ok(raw_packet) = packet.encode_packet(protocol_version) {
        client_data.write_packet(raw_packet).await?;
        client_data.shutdown().await?;
    }

    Ok(())
}

async fn send_keep_alive(client_data: &ClientData) -> Result<(), PacketProcessingError> {
    let (protocol_version, state) = {
        let client = client_data.client().await;
        (client.protocol_version(), client.state())
    };

    if state == State::Play {
        let packet = PacketRegistry::ClientBoundKeepAlive(ClientBoundKeepAlivePacket::random()?);
        let raw_packet = packet.encode_packet(protocol_version)?;
        client_data.write_packet(raw_packet).await?;
    }

    Ok(())
}
