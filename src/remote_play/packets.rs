use protocol::{Parcel, Error};
use std::io::prelude::*;
use std::{u8, u32};
use byteorder::{ByteOrder, WriteBytesExt, LittleEndian, BigEndian};

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

#[derive(Clone)]
pub struct RemotePlayControlPacket {
    magic: u32,
    sequence: u32,
    packet_type: u32,
    command: u32,
    args: [u32; 16],
}

impl Parcel for RemotePlayControlPacket {
    fn read(reader: &mut Read) -> Result<Self, Error> {
        panic!("read not supported for control packets")
    }

    fn write(&self, writer: &mut Write) -> Result<(), Error> {
        let mut buf = vec![];
        buf.write_u32::<LittleEndian>(self.magic);
        buf.write_u32::<LittleEndian>(self.sequence);
        buf.write_u32::<LittleEndian>(self.packet_type);
        buf.write_u32::<LittleEndian>(self.command);
        for i in &self.args {
            buf.write_u32::<LittleEndian>(*i);
        }
        buf.write_u32::<LittleEndian>(0);
        writer.write(&buf);
        Ok(())
    }
}

pub struct StreamingConfig {
    pub host: String,
    pub priority_screen: Screen,
    pub priority_factor: u32,
    pub compression_quality: u32,
    pub qos: u32
}


pub fn make_init_remote_play_packet(config: &StreamingConfig) -> RemotePlayControlPacket {
    const NTR_MAGIC: u32 = 0x12345678;

    let screen_bits: u32 = match config.priority_screen {
        Screen::Top => 1 << 8,
        Screen::Bottom => 0
    };

    let mode: u32 = screen_bits | config.priority_factor;

    let mut args: [u32; 16] = [0; 16];
    args[0] = mode;
    args[1] = config.compression_quality;
    args[2] = config.qos;

    RemotePlayControlPacket {
        magic: NTR_MAGIC,
        sequence: 3000,
        packet_type: 0,
        command: 901,
        args,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fmt::Write;

    #[test]
    fn make_packet() {
        let config = StreamingConfig {
            host: "0.0.0.0".to_string(),
            priority_screen: Screen::Top,
            priority_factor: 1,
            compression_quality:  75,
            qos: 1966080
        };
        let packet = make_init_remote_play_packet(&config);
        let bytes = packet.raw_bytes().unwrap();
        let mut hex = String::new();
        for &byte in bytes.iter() {
            write!(&mut hex, "{:02X}", byte);
        }
        assert_eq!(hex, "78563412B80B00000000000085030000010100004B00000000001E000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000");
    }
}