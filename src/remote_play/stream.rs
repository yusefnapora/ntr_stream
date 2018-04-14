use bus::{Bus, BusReader};
use protocol::wire;
use protocol::Parcel;
use remote_play::packets::{RemotePlayControlPacket, RemotePlayPacket, Screen, StreamingConfig};
use remote_play::packets::make_init_remote_play_packet;
use std::io;
use std::io::prelude::*;
use std::net::TcpStream;
use std::net::UdpSocket;
use std::sync::Arc;
use std::sync::Mutex;
use std::thread;
use std::time;

pub fn send_init_command(config: &StreamingConfig) {
    let packet = make_init_remote_play_packet(&config);
    let addr = format!("{}:8000", config.host);
    {
        let mut stream = TcpStream::connect(addr.clone()).expect("error connecting tcp stream");
        let bytes = packet.raw_bytes().expect("error getting control packet bytes");
        stream.write(&bytes).expect("error sending control packet");
        stream.flush();
        thread::sleep(time::Duration::from_secs(3));
    }
    let stream = TcpStream::connect(addr).expect("error connecting tcp stream");
}

struct ScreenPacketReaders {
    top: BusReader<RemotePlayPacket>,
    bottom: BusReader<RemotePlayPacket>
}

pub struct RemotePlayStream {
    socket: Arc<Mutex<UdpSocket>>,
    top_packet_bus: Arc<Mutex<Bus<RemotePlayPacket>>>,
    bottom_packet_bus: Arc<Mutex<Bus<RemotePlayPacket>>>,
    pub top_image_bus: Arc<Mutex<Bus<Vec<u8>>>>,
    pub bottom_image_bus: Arc<Mutex<Bus<Vec<u8>>>>
}

impl RemotePlayStream {
    pub fn bind() -> Result<RemotePlayStream, io::Error> {
        let socket = UdpSocket::bind("127.0.0.1:8001")?;
        let top_packet_bus = Arc::new(Mutex::new(Bus::new(10)));
        let bottom_packet_bus = Arc::new(Mutex::new(Bus::new(10)));
        let top_image_bus = Arc::new(Mutex::new(Bus::new(10)));
        let bottom_image_bus = Arc::new(Mutex::new(Bus::new(10)));

        let streamer = RemotePlayStream {
            socket: Arc::new(Mutex::new(socket)),
            top_packet_bus,
            bottom_packet_bus,
            top_image_bus,
            bottom_image_bus
        };
        streamer.stream();
        return Ok(streamer);
    }

    fn stream(&self) -> () {
//        let mut top_packet_reader = self.top_packet_bus.lock().unwrap().add_rx();
//        let mut bottom_packet_reader = self.bottom_packet_bus.lock().unwrap().add_rx();
        self.collect_remote_play_packets(Screen::Top);
        self.collect_remote_play_packets(Screen::Bottom);

        let socket = self.socket.clone();
        let top_packet_bus = self.top_packet_bus.clone();
        let bottom_packet_bus = self.bottom_packet_bus.clone();
        thread::spawn(move || {
            let mut pipeline = wire::dgram::Pipeline::new(wire::middleware::pipeline::default());
            loop {
                let mut buffer = [0u8; 2000];
                let bytes_read = socket.lock().unwrap().recv(&mut buffer).unwrap();
                let mut data = io::Cursor::new(&buffer[0..bytes_read]);
                let remote_play_packet: RemotePlayPacket = pipeline.receive_from(&mut data).unwrap();
                match remote_play_packet.screen() {
                    Screen::Top => {
                        let mut top_bus = top_packet_bus.lock().unwrap();
                        top_bus.broadcast(remote_play_packet)
                    },
                    Screen::Bottom => {
                        let mut bus = bottom_packet_bus.lock().unwrap();
                        bus.broadcast(remote_play_packet)
                    }
                };
            }
        });
    }

    fn collect_remote_play_packets(&self, screen: Screen) {
        let (frame_bus_arc, reader_arc) = match screen {
            Screen::Top => (self.top_image_bus.clone(), self.top_packet_bus.clone()),
            Screen::Bottom => (self.bottom_image_bus.clone(), self.bottom_packet_bus.clone())
        };

        let mut reader = reader_arc.lock().unwrap().add_rx();

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
                    let mut frame_bus = match frame_bus_arc.lock() {
                        Ok(t) => t,
                        Err(_) => continue
                    };
                    if frame_bus.try_broadcast(state.image_data).is_err() {
                        continue;
                    }
                } else {
                    frame_state = state.advance(packet);
                    // TODO: check if frame_state is None, log dropped packet
                }
            }
        });
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