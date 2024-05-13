use super::byte_packet_buffer::BytePacketBuffer;
use super::error::Result;
use super::query_type::QueryType;
use std::net::{Ipv4Addr, Ipv6Addr};

///                               1  1  1  1  1  1
/// 0  1  2  3  4  5  6  7  8  9  0  1  2  3  4  5
/// +--+--+--+--+--+--+--+--+--+--+--+--+--+--+--+--+
/// |                                               |
/// /                                               /
/// /                      NAME                     /
/// |                                               |
/// +--+--+--+--+--+--+--+--+--+--+--+--+--+--+--+--+
/// |                      TYPE                     |
/// +--+--+--+--+--+--+--+--+--+--+--+--+--+--+--+--+
/// |                     CLASS                     |
/// +--+--+--+--+--+--+--+--+--+--+--+--+--+--+--+--+
/// |                      TTL                      |
/// |                                               |
/// +--+--+--+--+--+--+--+--+--+--+--+--+--+--+--+--+
/// |                   RDLENGTH                    |
/// +--+--+--+--+--+--+--+--+--+--+--+--+--+--+--+--|
/// /                     RDATA                     /
/// /                                               /
/// +--+--+--+--+--+--+--+--+--+--+--+--+--+--+--+--+
#[derive(Debug)]
pub struct Record {
    pub name: String,
    pub qtype: QueryType,
    pub ttl: u32,
    pub rdlength: u16,
    pub rdata: RData,
}

#[derive(Debug)]
pub enum RData {
    Unknown,
    A(Ipv4Addr),
    AAAA(Ipv6Addr),
}

impl Record {
    pub fn read(buf: &mut BytePacketBuffer) -> super::error::Result<Self> {
        let name = buf.read_qname()?;
        let qtype = QueryType::from(buf.read_u16()?);
        let _ = buf.read_u16()?;
        let ttl = buf.read_u32()?;
        let rdlen = buf.read_u16()?;
        let rdata = match qtype {
            QueryType::A => {
                let addr = buf.read_u32()?;
                let addr = Ipv4Addr::new(
                    ((addr >> 24) & 0xFF) as u8,
                    ((addr >> 16) & 0xFF) as u8,
                    ((addr >> 8) & 0xFF) as u8,
                    (addr & 0xFF) as u8,
                );
                RData::A(addr)
            }
            QueryType::AAAA => {
                let addr1 = buf.read_u32()?;
                let addr2 = buf.read_u32()?;
                let addr3 = buf.read_u32()?;
                let addr4 = buf.read_u32()?;
                let addr = Ipv6Addr::new(
                    ((addr1 >> 16) & 0xFFFF) as u16,
                    (addr1 & 0xFFFF) as u16,
                    ((addr2 >> 16) & 0xFFFF) as u16,
                    (addr2 & 0xFFFF) as u16,
                    ((addr3 >> 16) & 0xFFFF) as u16,
                    (addr3 & 0xFFFF) as u16,
                    ((addr4 >> 16) & 0xFFFF) as u16,
                    (addr4 & 0xFFFF) as u16,
                );
                RData::AAAA(addr)
            }
            _ => {
                buf.step(rdlen as usize)?;
                RData::Unknown
            }
        };

        Ok(Record {
            name,
            qtype,
            ttl,
            rdlength: rdlen,
            rdata,
        })
    }

    pub fn write(&self, buf: &mut BytePacketBuffer) -> Result<usize> {
        let p = buf.pos();
        match self.rdata {
            RData::A(v) => {
                buf.write_qname(&self.name)?;
                buf.write_u16(QueryType::A.into())?;
                buf.write_u16(1)?;
                buf.write_u32(self.ttl)?;
                buf.write_u16(4)?;
                let o = v.octets();
                buf.write_u8(o[0])?;
                buf.write_u8(o[1])?;
                buf.write_u8(o[2])?;
                buf.write_u8(o[3])?;
            }
            RData::AAAA(v) => {
                buf.write_qname(&self.name)?;
                buf.write_u16(QueryType::AAAA.into())?;
                buf.write_u16(1)?;
                buf.write_u32(self.ttl)?;
                buf.write_u16(16)?;

                for octet in &v.segments() {
                    buf.write_u16(*octet)?;
                }
            }
            _ => (),
        }
        Ok(buf.pos() - p)
    }
}
