use anyhow::Result;
use std::io::Read;

pub fn generate_packet(operation: u32, payload: &[u8]) -> Vec<u8> {
    let packet_len = (payload.len() + 16) as u32;
    let mut buffer = Vec::with_capacity(payload.len() + 16);
    buffer.extend_from_slice(&packet_len.to_be_bytes());
    buffer.extend_from_slice(&16u16.to_be_bytes());
    buffer.extend_from_slice(&1u16.to_be_bytes());
    buffer.extend_from_slice(&operation.to_be_bytes());
    buffer.extend_from_slice(&1u32.to_be_bytes());
    buffer.extend_from_slice(payload);
    buffer
}

#[derive(Debug)]
pub enum DecodedPacket {
    Popular { count: u32 },
    AuthOk,
    Message(serde_json::Value),
}

pub fn decode_packets(buffer: &[u8]) -> Vec<DecodedPacket> {
    let mut packets = Vec::new();
    decode_into(buffer, &mut packets);
    packets
}

fn decode_into(buffer: &[u8], packets: &mut Vec<DecodedPacket>) {
    let mut cursor = 0usize;
    while cursor + 16 <= buffer.len() {
        let packet_len = read_u32(buffer, cursor) as usize;
        let header_len = read_u16(buffer, cursor + 4) as usize;
        let version = read_u16(buffer, cursor + 6);
        let operation = read_u32(buffer, cursor + 8);
        if packet_len < 16 || cursor + packet_len > buffer.len() || header_len < 16 {
            break;
        }
        let body = &buffer[cursor + header_len..cursor + packet_len];
        match operation {
            3 => {
                if body.len() >= 4 {
                    let count = u32::from_be_bytes([body[0], body[1], body[2], body[3]]);
                    packets.push(DecodedPacket::Popular { count });
                }
            }
            8 => packets.push(DecodedPacket::AuthOk),
            5 => match version {
                2 => {
                    if let Ok(unzipped) = inflate_zlib(body) {
                        decode_into(&unzipped, packets);
                    }
                }
                3 => {
                    if let Ok(unzipped) = inflate_brotli(body) {
                        decode_into(&unzipped, packets);
                    }
                }
                _ => {
                    if let Ok(text) = std::str::from_utf8(body) {
                        if let Ok(json) = serde_json::from_str::<serde_json::Value>(text) {
                            packets.push(DecodedPacket::Message(json));
                        }
                    }
                }
            },
            _ => {}
        }
        cursor += packet_len;
    }
}

fn read_u16(buffer: &[u8], offset: usize) -> u16 {
    u16::from_be_bytes([buffer[offset], buffer[offset + 1]])
}

fn read_u32(buffer: &[u8], offset: usize) -> u32 {
    u32::from_be_bytes([
        buffer[offset],
        buffer[offset + 1],
        buffer[offset + 2],
        buffer[offset + 3],
    ])
}

fn inflate_zlib(input: &[u8]) -> Result<Vec<u8>> {
    let mut decoder = flate2::read::ZlibDecoder::new(input);
    let mut output = Vec::new();
    decoder.read_to_end(&mut output)?;
    Ok(output)
}

fn inflate_brotli(input: &[u8]) -> Result<Vec<u8>> {
    let mut decoder = brotli::Decompressor::new(input, 4096);
    let mut output = Vec::new();
    decoder.read_to_end(&mut output)?;
    Ok(output)
}
