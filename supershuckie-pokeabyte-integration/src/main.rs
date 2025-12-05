use std::time::Duration;
use supershuckie_pokeabyte_integration::PokeAByteIntegrationServer;

// Simulate an integration server where all bytes count up by 1.
fn main() {
    let server = PokeAByteIntegrationServer::begin_listen().unwrap();
    let mut q = 0u8;

    loop {
        {
            let mut memory_lock = server.get_memory().lock().unwrap();
            let memory_option = memory_lock.as_mut();
            if let Some(memory) = memory_option  {
                unsafe { memory.get_memory_mut() }.fill(q);
                q = q.wrapping_add(1);
            }
        }
        std::thread::sleep(Duration::from_secs(1));
    }
}
