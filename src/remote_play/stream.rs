use protocol::wire;
use bus::{Bus, BusReader};
use std::io;
use std::thread;
use std::net::UdpSocket;
use remote_play::packets::RemotePlayPacket;
use remote_play::packets::Screen;

struct ScreenPacketReaders {
    top: BusReader<RemotePlayPacket>,
    bottom: BusReader<RemotePlayPacket>
}

pub struct ImageReaders {
    pub top: BusReader<Vec<u8>>,
    pub bottom: BusReader<Vec<u8>>
}

pub fn remote_play_stream() -> ImageReaders {
    let socket = UdpSocket::bind("127.0.0.1:8000")
        .expect("Unable to bind to UDP port 8000");

    let mut pipeline = wire::dgram::Pipeline::new(wire::middleware::pipeline::default());
    let mut top_bus = Bus::new(10);
    let mut bottom_bus = Bus::new(10);
    let mut top_packet_reader = top_bus.add_rx();
    let mut bottom_packet_reader = bottom_bus.add_rx();

    thread::spawn(move || {
        loop {
            let mut buffer = [0u8; 2000];
            let bytes_read = socket.recv(&mut buffer).unwrap();
            let mut data = io::Cursor::new(&buffer[0..bytes_read]);
            let remote_play_packet: RemotePlayPacket = pipeline.receive_from(&mut data).unwrap();
            match remote_play_packet.screen() {
                Screen::Top => top_bus.broadcast(remote_play_packet),
                Screen::Bottom => bottom_bus.broadcast(remote_play_packet)
            };
        }
    });

    ImageReaders {
        top: collect_remote_play_packets(top_packet_reader),
        bottom: collect_remote_play_packets(bottom_packet_reader)
    }
}


struct FrameState {
    frame_id: u8,
    next_packet_id: u8,
    frame_complete: bool,
    image_data: Vec<u8>,
}

impl FrameState {
    fn from_initial_packet(packet: RemotePlayPacket) -> Option<FrameState> {
        if packet.packet_id != 0 {
            None
        } else {
            let mut data: Vec<u8> = Vec::with_capacity(packet.image_data.len());
            data.extend(packet.image_data.iter());
            Some(FrameState {
                frame_id: packet.frame_id,
                next_packet_id: packet.packet_id + 1,
                frame_complete: false,
                image_data: data })
        }
    }

    fn advance(mut self, packet: RemotePlayPacket) -> Option<FrameState> {
        if packet.frame_id != self.frame_id && !self.frame_complete {
            return None;
        }
        if packet.packet_id != self.next_packet_id {
            return None;
        }
        self.image_data.extend(packet.image_data.iter());
        Some(FrameState {
            frame_id: packet.frame_id,
            next_packet_id: packet.packet_id + 1,
            frame_complete: packet.is_end_of_frame(),
            image_data: self.image_data
        })
    }
}

fn collect_remote_play_packets(mut reader: BusReader<RemotePlayPacket>) -> BusReader<Vec<u8>> {
    let mut frame_bus = Bus::new(10);
    let frame_reader = frame_bus.add_rx();

    thread::spawn(move || {
        let mut frame_state: Option<FrameState> = None;
        for packet in reader.iter() {
            if frame_state.is_none() {
                frame_state = FrameState::from_initial_packet(packet);
                continue;
            }

            let mut state = frame_state.unwrap();
            if state.frame_complete {
                frame_state = None;
                if frame_bus.try_broadcast(state.image_data).is_err() {
                    continue;
                }
            } else {
                frame_state = state.advance(packet);
                // TODO: check if frame_state is None, log dropped packet
            }
        }
    });

    frame_reader
}