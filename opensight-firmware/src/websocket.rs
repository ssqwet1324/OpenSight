use alloc::format;
use embassy_net::{Ipv4Address, Stack, tcp::TcpSocket};
use embassy_time::{Duration, Timer, with_timeout};
use embedded_websocket::{
    Error as CodecError, WebSocketClient, WebSocketOptions,
    WebSocketReceiveMessageType as Received, WebSocketSendMessageType as Sent,
};
use esp_hal::rng::Rng;
use esp_println::println;
use rand_core::RngCore;

use crate::server_config;

const MAX_ANSWER: usize = 4096;

struct HardwareRng(Rng);

impl RngCore for HardwareRng {
    fn next_u32(&mut self) -> u32 {
        self.0.random()
    }
    fn next_u64(&mut self) -> u64 {
        ((self.next_u32() as u64) << 32) | self.next_u32() as u64
    }
    fn fill_bytes(&mut self, dest: &mut [u8]) {
        for chunk in dest.chunks_mut(4) {
            let value = self.next_u32().to_le_bytes();
            chunk.copy_from_slice(&value[..chunk.len()]);
        }
    }
    fn try_fill_bytes(&mut self, dest: &mut [u8]) -> Result<(), rand_core::Error> {
        self.fill_bytes(dest);
        Ok(())
    }
}

async fn write_all(socket: &mut TcpSocket<'_>, mut data: &[u8]) -> Result<(), &'static str> {
    while !data.is_empty() {
        let count = socket.write(data).await.map_err(|_| "TCP write failed")?;
        if count == 0 {
            return Err("TCP write closed");
        }
        data = &data[count..];
    }
    Ok(())
}

async fn session(stack: Stack<'static>) -> Result<(), &'static str> {
    let address: Ipv4Address = server_config::HOST
        .parse()
        .map_err(|_| "Invalid Go server IP")?;
    let mut tcp_rx = [0u8; 2048];
    let mut tcp_tx = [0u8; 1024];
    let mut socket = TcpSocket::new(stack, &mut tcp_rx, &mut tcp_tx);
    socket.set_timeout(Some(Duration::from_secs(60)));
    socket.set_keep_alive(Some(Duration::from_secs(15)));

    println!(
        "Connecting to Go WebSocket at {}:{}...",
        address,
        server_config::PORT
    );
    with_timeout(
        Duration::from_secs(10),
        socket.connect((address, server_config::PORT)),
    )
    .await
    .map_err(|_| "TCP connect timeout")?
    .map_err(|_| "Go server is unavailable")?;

    let mut codec = WebSocketClient::new_client(HardwareRng(Rng::new()));
    let host = format!("{}:{}", server_config::HOST, server_config::PORT);
    let origin = format!("http://{}", host);
    let options = WebSocketOptions {
        path: "/api/v1/ws",
        host: &host,
        origin: &origin,
        sub_protocols: None,
        additional_headers: None,
    };
    let mut wire = [0u8; 2048];
    let (request_len, key) = codec
        .client_connect(&options, &mut wire)
        .map_err(|_| "Failed to create WebSocket handshake")?;
    write_all(&mut socket, &wire[..request_len]).await?;

    // A TCP read may contain part of the HTTP header, or the header plus the first answer.
    let mut available = 0;
    let handshake_len = with_timeout(Duration::from_secs(10), async {
        loop {
            if available == wire.len() {
                return Err("WebSocket handshake too large");
            }
            let count = socket
                .read(&mut wire[available..])
                .await
                .map_err(|_| "Handshake read failed")?;
            if count == 0 {
                return Err("Go closed the handshake");
            }
            available += count;
            match codec.client_accept(&key, &wire[..available]) {
                Ok((len, _)) => return Ok(len),
                Err(CodecError::HttpHeaderIncomplete) => continue,
                Err(_) => return Err("Go rejected WebSocket handshake"),
            }
        }
    })
    .await
    .map_err(|_| "WebSocket handshake timeout")??;
    wire.copy_within(handshake_len..available, 0);
    available -= handshake_len;
    println!("WebSocket connected; waiting for Qwen answers.");

    let mut piece = [0u8; 1024];
    let mut answer = [0u8; MAX_ANSWER];
    let mut answer_len = 0;
    let mut control = [0u8; 125];
    let mut control_len = 0;
    let mut control_tx = [0u8; 160];

    loop {
        let mut consumed = 0;
        while consumed < available {
            let result = match codec.read(&wire[consumed..available], &mut piece) {
                Ok(result) => result,
                Err(CodecError::ReadFrameIncomplete) => break,
                Err(_) => return Err("Invalid WebSocket frame"),
            };
            if result.len_from == 0 {
                return Err("WebSocket decoder made no progress");
            }
            consumed += result.len_from;
            match result.message_type {
                Received::Text => {
                    if answer_len + result.len_to > answer.len() {
                        return Err("Answer exceeds display buffer");
                    }
                    answer[answer_len..answer_len + result.len_to]
                        .copy_from_slice(&piece[..result.len_to]);
                    answer_len += result.len_to;
                    if result.end_of_message {
                        let text = core::str::from_utf8(&answer[..answer_len])
                            .map_err(|_| "Invalid answer UTF-8")?;
                        if !text.starts_with("\x1eOSF1 ") {
                            println!("Qwen: {}", text);
                        }
                        crate::oled::publish(text);
                        answer_len = 0;
                    }
                }
                Received::Ping | Received::Pong | Received::CloseMustReply => {
                    if control_len + result.len_to > control.len() {
                        return Err("Invalid control frame");
                    }
                    control[control_len..control_len + result.len_to]
                        .copy_from_slice(&piece[..result.len_to]);
                    control_len += result.len_to;
                    if result.end_of_message {
                        if result.message_type == Received::Ping {
                            let len = codec
                                .write(Sent::Pong, true, &control[..control_len], &mut control_tx)
                                .map_err(|_| "Failed to encode pong")?;
                            write_all(&mut socket, &control_tx[..len]).await?;
                        } else if result.message_type == Received::CloseMustReply {
                            // An empty close reply is valid and completes the server's closing handshake.
                            let len = codec
                                .write(Sent::CloseReply, true, &[], &mut control_tx)
                                .map_err(|_| "Failed to encode close reply")?;
                            write_all(&mut socket, &control_tx[..len]).await?;
                            socket.flush().await.map_err(|_| "Close flush failed")?;
                            return Err("Go closed the WebSocket");
                        }
                        control_len = 0;
                    }
                }
                Received::CloseCompleted => return Err("WebSocket closed"),
                Received::Binary => return Err("Expected a text answer"),
            }
        }
        wire.copy_within(consumed..available, 0);
        available -= consumed;
        if available == wire.len() {
            return Err("WebSocket frame header too large");
        }
        let count = with_timeout(Duration::from_secs(50), socket.read(&mut wire[available..]))
            .await
            .map_err(|_| "No WebSocket heartbeat from Go")?
            .map_err(|_| "WebSocket connection lost")?;
        if count == 0 {
            return Err("Go disconnected");
        }
        available += count;
    }
}

#[embassy_executor::task]
pub async fn receiver(stack: Stack<'static>) {
    loop {
        stack.wait_config_up().await;
        if let Err(error) = session(stack).await {
            println!("WebSocket: {}. Retry in 5 seconds.", error);
        }
        Timer::after(Duration::from_secs(5)).await;
    }
}
