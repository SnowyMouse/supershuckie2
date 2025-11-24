use std::borrow::Cow;
use std::net::UdpSocket;
use std::sync::{Arc, Mutex, RwLock, Weak};
use std::time::Duration;
use tinyvec::{ArrayVec, TinyVec};
use crate::protocol::{Instruction, MetadataHeader, PokeAByteProtocolRequestPacket, PokeAByteProtocolRequestReadBlock, MAX_NUMBER_OF_READ_BLOCKS};
use crate::shared_memory::PokeAByteSharedMemory;

#[cfg(not(target_pointer_width = "64"))]
compile_error!("must be compiled for 64-bit");

// FIXME: this is not currently configurable
const POKEABYTE_UDP: &str = "127.0.0.1:55356";

pub struct PokeAByteWrite {
    pub address: u32,
    pub data: TinyVec<[u8; 16]>
}

pub struct PokeAByteIntegrationServer {
    socket: UdpSocket,
    shared_memory: Mutex<PokeAByteSharedMemory>,
    writes: Mutex<Vec<PokeAByteWrite>>,
    setup: RwLock<Option<Arc<PokeAByteSetup>>>
}

pub struct PokeAByteSetup {
    pub blocks: ArrayVec<[PokeAByteProtocolRequestReadBlock; MAX_NUMBER_OF_READ_BLOCKS]>,
    pub frame_skip: Option<u32>
}

impl PokeAByteIntegrationServer {
    pub fn begin_listen() -> Result<Arc<Self>, PokeAByteError> {
        let socket = UdpSocket::bind(&POKEABYTE_UDP)
            .map_err(|e| PokeAByteError::SocketFailure { explanation: Cow::Owned(format!("Failed to bind: {e:?}")) })?;

        let _ = socket.set_read_timeout(Some(Duration::from_secs(1)));
        let _ = socket.set_write_timeout(Some(Duration::from_secs(1)));

        let this = Arc::new(Self {
            socket,
            shared_memory: Mutex::new(PokeAByteSharedMemory::new()?),
            setup: RwLock::new(None),
            writes: Mutex::new(Vec::with_capacity(64))
        });

        let this_downgraded = Arc::downgrade(&this);

        let _ = std::thread::Builder::new().name("PokeAByteIntegrationServer".to_owned()).spawn(move || {
            PokeAByteIntegrationServer::thread(this_downgraded)
        });

        Ok(this)
    }

    pub fn get_setup(&self) -> Option<Arc<PokeAByteSetup>> {
        self.setup.read().expect("failed to read blocks").clone()
    }

    fn thread(this: Weak<PokeAByteIntegrationServer>) {
        let mut buffer = vec![0u8; 65536];

        loop {
            let Some(promotion) = this.upgrade() else {
                return
            };

            let Ok((len, addr)) = promotion.socket.recv_from(&mut buffer) else {
                continue
            };

            let bytes_received = &buffer.as_slice()[..len];
            let packet = match PokeAByteProtocolRequestPacket::parse_bytes(bytes_received) {
                Ok(n) => n,
                Err(e) => {
                    // TODO: should we log this?
                    if cfg!(debug_assertions) {
                        eprintln!("PokeAByte error: {e:?}");
                    }
                    continue
                }
            };

            match packet {
                PokeAByteProtocolRequestPacket::Ping => {
                    let _ = promotion.socket.send_to(&MetadataHeader::new_response(Instruction::Ping).into_bytes(), addr);
                },
                PokeAByteProtocolRequestPacket::NoOp => {},
                PokeAByteProtocolRequestPacket::Close => {
                    // unhandled for now
                },
                PokeAByteProtocolRequestPacket::Setup { blocks, frame_skip } => {
                    let _ = promotion.socket.send_to(&MetadataHeader::new_response(Instruction::Setup).into_bytes(), addr);
                    *promotion.setup.write().expect("failed to write to blocks") = Some(Arc::new(PokeAByteSetup {
                        blocks, frame_skip
                    }));
                },
                PokeAByteProtocolRequestPacket::Write { data, address } => {
                    if data.is_empty() {
                        continue
                    }

                    promotion.writes.lock().expect("failed to write to writes").push(PokeAByteWrite {
                        address, data: data.into()
                    })
                }
            }
        }
    }

    pub fn get_writes(&self) -> &Mutex<Vec<PokeAByteWrite>> {
        &self.writes
    }

    pub fn get_memory(&self) -> &Mutex<PokeAByteSharedMemory> {
        &self.shared_memory
    }
}

#[derive(Clone, PartialEq, Debug)]
pub enum PokeAByteError {
    SharedMemoryFailure { explanation: Cow<'static, str> },
    SocketFailure { explanation: Cow<'static, str> },
    BadPacketFromClient { explanation: Cow<'static, str> }
}

mod shared_memory;
mod protocol;
