use super::byte_packet_buffer::BytePacketBuffer;
use super::error::Result;
use super::query_type::QueryType;
use std::fmt;
use std::net::{Ipv4Addr, Ipv6Addr};

//                               1  1  1  1  1  1
// 0  1  2  3  4  5  6  7  8  9  0  1  2  3  4  5
// +--+--+--+--+--+--+--+--+--+--+--+--+--+--+--+--+
// |                                               |
// /                                               /
// /                      NAME                     /
// |                                               |
// +--+--+--+--+--+--+--+--+--+--+--+--+--+--+--+--+
// |                      TYPE                     |
// +--+--+--+--+--+--+--+--+--+--+--+--+--+--+--+--+
// |                     CLASS                     |
// +--+--+--+--+--+--+--+--+--+--+--+--+--+--+--+--+
// |                      TTL                      |
// |                                               |
// +--+--+--+--+--+--+--+--+--+--+--+--+--+--+--+--+
// |                   RDLENGTH                    |
// +--+--+--+--+--+--+--+--+--+--+--+--+--+--+--+--|
// /                     RDATA                     /
// /                                               /
// +--+--+--+--+--+--+--+--+--+--+--+--+--+--+--+--+
#[derive(Debug)]
pub struct Record {
    pub name: String,
    pub qtype: QueryType,
    pub class: u16,
    pub ttl: u32,
    pub rdlength: u16,
    pub rdata: RData,
}

#[derive(Debug)]
pub enum RData {
    Unknown(QueryType, Vec<u8>),
    A(Ipv4Addr),
    AAAA(Ipv6Addr),
    CNAME(u16, String, Vec<u8>),
    SRV(u16, SrvRecord, Vec<u8>),
}

impl Record {
    pub fn read(buf: &mut BytePacketBuffer) -> super::error::Result<Self> {
        let name = buf.read_qname()?;
        let qtype = QueryType::from(buf.read_u16()?);
        let class = buf.read_u16()?;
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
            QueryType::CNAME => {
                let v = buf.get_range(buf.pos(), rdlen as usize)?.to_vec();
                let name = buf.read_qname()?;
                RData::CNAME(rdlen, name, v)
            }
            QueryType::SRV => {
                let v = buf.get_range(buf.pos(), rdlen as usize)?.to_vec();
                let priority = buf.read_u16()?;
                let weight = buf.read_u16()?;
                let port = buf.read_u16()?;
                let target = buf.read_qname()?;
                RData::SRV(rdlen, SrvRecord::new(priority, weight, port, target), v)
            }
            _ => {
                let v = buf.read_range(rdlen as usize)?;
                RData::Unknown(qtype, v.to_vec())
            }
        };

        Ok(Record {
            name,
            qtype,
            class,
            ttl,
            rdlength: rdlen,
            rdata,
        })
    }

    pub fn write(&self, buf: &mut BytePacketBuffer) -> Result<usize> {
        let p = buf.pos();
        match &self.rdata {
            RData::A(v) => {
                buf.write_qname(&self.name)?;
                buf.write_u16(QueryType::A.into())?;
                buf.write_u16(self.class)?;
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
                buf.write_u16(self.class)?;
                buf.write_u32(self.ttl)?;
                buf.write_u16(16)?;

                for octet in &v.segments() {
                    buf.write_u16(*octet)?;
                }
            }
            RData::CNAME(len, _name, v) => {
                buf.write_qname(&self.name)?;
                buf.write_u16(QueryType::CNAME.into())?;
                buf.write_u16(self.class)?;
                buf.write_u32(self.ttl)?;
                buf.write_u16(*len)?;
                buf.write_range(v)?;
            }
            RData::SRV(len, _srv, v) => {
                buf.write_qname(&self.name)?;
                buf.write_u16(QueryType::SRV.into())?;
                buf.write_u16(self.class)?;
                buf.write_u32(self.ttl)?;
                buf.write_u16(*len)?;
                buf.write_range(v)?;
            }
            RData::Unknown(qtype, v) => {
                buf.write_qname(&self.name)?;
                buf.write_u16((*qtype).into())?;
                buf.write_u16(self.class)?;
                buf.write_u32(self.ttl)?;
                buf.write_u16(v.len() as u16)?;
                buf.write_range(v)?;
            }
        }
        Ok(buf.pos() - p)
    }

    pub fn debug_fmt(&self, f: &mut std::fmt::Formatter<'_>, indent: usize) -> std::fmt::Result {
        let t = "\t".repeat(indent);
        writeln!(f, "{t}name: {}", self.name)?;
        writeln!(f, "{t}qtype: {}", self.qtype)?;
        writeln!(f, "{t}class: {}", self.class)?;
        writeln!(f, "{t}ttl: {}", self.ttl)?;
        writeln!(f, "{t}rdlength: {}", self.rdlength)?;
        match &self.rdata {
            RData::A(v) => writeln!(f, "{t}{v}")?,
            RData::AAAA(v) => writeln!(f, "{t}{v}")?,
            RData::CNAME(_, v, _) => writeln!(f, "{t}{v}")?,
            RData::SRV(_, v, _) => v.debug_fmt(f, indent)?,
            RData::Unknown(_, v) => writeln!(f, "len: {}", v.len())?,
        }
        Ok(())
    }
}

#[derive(Debug)]
pub struct SrvRecord {
    pub priority: u16,
    pub weight: u16,
    pub port: u16,
    pub target: String,
}

impl SrvRecord {
    fn new(priority: u16, weight: u16, port: u16, target: impl Into<String>) -> Self {
        Self {
            priority,
            weight,
            port,
            target: target.into(),
        }
    }

    pub fn debug_fmt(&self, f: &mut std::fmt::Formatter<'_>, indent: usize) -> std::fmt::Result {
        let t = "\t".repeat(indent);
        writeln!(f, "{t}priority: {}", self.priority)?;
        writeln!(f, "{t}weight: {}", self.weight)?;
        writeln!(f, "{t}port: {}", self.port)?;
        writeln!(f, "{t}target: {}", self.target)?;
        Ok(())
    }
}

impl fmt::Display for SrvRecord {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} {} {} {}",
            self.priority, self.weight, self.port, self.target
        )
    }
}
