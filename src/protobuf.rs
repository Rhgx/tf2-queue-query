use anyhow::{Context, Result, bail};

pub const PROTO_MASK: u32 = 0x8000_0000;

pub fn append_varint(output: &mut Vec<u8>, mut value: u64) {
    loop {
        let mut byte = (value & 0x7f) as u8;
        value >>= 7;
        if value != 0 {
            byte |= 0x80;
        }
        output.push(byte);
        if value == 0 {
            break;
        }
    }
}

pub fn proto_packet(message_type: u32, body: &[u8]) -> Vec<u8> {
    let mut packet = Vec::with_capacity(8 + body.len());
    packet.extend_from_slice(&(message_type | PROTO_MASK).to_le_bytes());
    packet.extend_from_slice(&0_u32.to_le_bytes());
    packet.extend_from_slice(body);
    packet
}

pub fn tf_client_init(version: u32) -> Vec<u8> {
    let mut body = vec![0x08];
    append_varint(&mut body, u64::from(version));
    body.extend_from_slice(&[0x10, 0x00]);
    body
}

pub fn body_from_packet(raw_type: u32, packet: &[u8]) -> &[u8] {
    if raw_type & PROTO_MASK == 0 || packet.len() < 8 {
        return packet;
    }
    let embedded = u32::from_le_bytes(packet[0..4].try_into().expect("four bytes"));
    if embedded != raw_type {
        return packet;
    }
    let header_size = u32::from_le_bytes(packet[4..8].try_into().expect("four bytes")) as usize;
    packet.get(8 + header_size..).unwrap_or(packet)
}

fn read_varint(bytes: &[u8], offset: &mut usize) -> Result<u64> {
    let mut result = 0_u64;
    for shift in (0..64).step_by(7) {
        let byte = *bytes.get(*offset).context("truncated protobuf varint")?;
        *offset += 1;
        if shift == 63 && byte > 1 {
            bail!("protobuf varint exceeds uint64 range");
        }
        result |= u64::from(byte & 0x7f) << shift;
        if byte & 0x80 == 0 {
            return Ok(result);
        }
    }
    bail!("malformed protobuf varint")
}

fn skip_field(bytes: &[u8], wire_type: u64, offset: &mut usize) -> Result<()> {
    match wire_type {
        0 => {
            read_varint(bytes, offset)?;
        }
        1 => *offset = offset.checked_add(8).context("protobuf offset overflow")?,
        2 => {
            let length = usize::try_from(read_varint(bytes, offset)?)
                .context("protobuf field is too large")?;
            *offset = offset
                .checked_add(length)
                .context("protobuf offset overflow")?;
        }
        5 => *offset = offset.checked_add(4).context("protobuf offset overflow")?,
        _ => bail!("unsupported protobuf wire type {wire_type}"),
    }
    if *offset > bytes.len() {
        bail!("truncated protobuf field");
    }
    Ok(())
}

/// Decode GC message 6525's packed or legacy-unpacked `map_count` field.
pub fn decode_map_counts(bytes: &[u8]) -> Result<Vec<u32>> {
    let mut counts = Vec::new();
    let mut offset = 0;
    while offset < bytes.len() {
        let tag = read_varint(bytes, &mut offset)?;
        let field = tag >> 3;
        let wire_type = tag & 7;
        match (field, wire_type) {
            (1, 0) => counts.push(
                u32::try_from(read_varint(bytes, &mut offset)?)
                    .context("map_count is outside uint32 range")?,
            ),
            (1, 2) => {
                let length = usize::try_from(read_varint(bytes, &mut offset)?)
                    .context("packed map_count is too large")?;
                let end = offset
                    .checked_add(length)
                    .context("protobuf offset overflow")?;
                if end > bytes.len() {
                    bail!("truncated packed map_count field");
                }
                while offset < end {
                    counts.push(
                        u32::try_from(read_varint(bytes, &mut offset)?)
                            .context("map_count is outside uint32 range")?,
                    );
                }
            }
            _ => skip_field(bytes, wire_type, &mut offset)?,
        }
    }
    if counts.is_empty() {
        bail!("GC response did not contain any map_count values");
    }
    Ok(counts)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_packed_counts() {
        assert_eq!(
            decode_map_counts(&[0x0a, 0x04, 0x00, 0x01, 0xac, 0x02]).unwrap(),
            [0, 1, 300]
        );
    }

    #[test]
    fn decodes_unpacked_counts_and_skips_unknown() {
        assert_eq!(
            decode_map_counts(&[0x10, 0x07, 0x08, 0x04, 0x08, 0x05]).unwrap(),
            [4, 5]
        );
    }

    #[test]
    fn strips_gc_proto_header() {
        let packet = proto_packet(6525, &[1, 2, 3]);
        assert_eq!(body_from_packet(0x197d | PROTO_MASK, &packet), [1, 2, 3]);
    }
}
