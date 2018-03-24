use protocol::{Parcel, Error};
use std::io::prelude::*;
use std::u8;

#[derive(Clone, PartialEq)]
pub enum Screen {
    Top,
    Bottom
}

#[derive(Clone)]
pub struct RemotePlayPacket {
    pub frame_id: u8,
    pub flags: u8,
    pub format: u8,
    pub packet_id: u8,
    pub image_data: [u8; 1444]
}

impl Parcel for RemotePlayPacket {
    fn read(reader: &mut Read) -> Result<Self, Error> {
        let frame_id = u8::read(reader)?;
        let flags = u8::read(reader)?;
        let format = u8::read(reader)?;
        let packet_id = u8::read(reader)?;
        let mut image_data: [u8; 1444] = [0; 1444];
        reader.read(&mut image_data)?;
        Ok(RemotePlayPacket { frame_id, flags, format, packet_id, image_data})
    }

    fn write(&self, writer: &mut Write) -> Result<(), Error> {
        self.frame_id.write(writer)?;
        self.flags.write(writer)?;
        self.format.write(writer)?;
        self.packet_id.write(writer)?;
        writer.write(&self.image_data)?;
        Ok(())
    }
}

impl RemotePlayPacket {
    pub fn is_end_of_frame(&self) -> bool {
        ((self.flags & 0x0f) >> 4) == 1
    }

    pub fn screen(&self) -> Screen {
        match self.flags & 0x0f {
            1 => Screen::Top,
            _ => Screen::Bottom
        }
    }
}

#[derive(Protocol, Clone)]
pub struct NtrControlPacket {
    magic: u32,
    unknown: u32,
    packet_type: u32,
    command: u32,
    args: [u32; 16],
    data_length: u32
}


pub struct StreamingConfig {
    priority_screen: Screen,
    priority_factor: u32,
    compression_quality: u32,
    qos_kbps: f64
}


fn make_init_remote_play_packet(config: &StreamingConfig) -> NtrControlPacket {
    const NTR_MAGIC: u32 = 0x12345678;
    let screen_bits: u32 = match config.priority_screen {
        Screen::Top => 1 << 8,
        Screen::Bottom => 0
    };

    let mode: u32 = screen_bits | config.priority_factor;
    let qos_bytes: u32 = (config.qos_kbps * 1024. * 1024.) as u32;

    let mut args: [u32; 16] = [0; 16];
    args[0] = mode;
    args[1] = config.compression_quality;
    args[2] = qos_bytes;

    NtrControlPacket {
        magic: NTR_MAGIC,
        unknown: 1,
        packet_type: 0,
        command: 901,
        args,
        data_length: 0
    }
}
