use std::cell::OnceCell;
use std::ffi::{c_char, CStr};
use std::mem::{transmute, zeroed};
use std::ptr::null;
use std::sync::{Arc, Once, OnceLock};
use sdl3_main::state::AppState;
use sdl3_sys::everything::SDL_Vulkan_GetInstanceExtensions;
use sdl3_sys::video::SDL_Window;
use sdl3_sys::vulkan::{SDL_Vulkan_CreateSurface, SDL_Vulkan_GetPresentationSupport, SDL_Vulkan_LoadLibrary};
use vulkano::instance::{Instance, InstanceCreateInfo, InstanceExtensions};
use vulkano::{Version, VulkanLibrary, VulkanObject};
use vulkano::command_buffer::allocator::StandardCommandBufferAllocator;
use vulkano::device::{Device, DeviceCreateInfo, DeviceExtensions, DeviceFeatures, Queue, QueueCreateFlags, QueueCreateInfo, QueueFamilyProperties, QueueFlags};
use vulkano::device::physical::{PhysicalDevice, PhysicalDeviceType};
use vulkano::memory::allocator::{FreeListAllocator, GenericMemoryAllocator, GenericMemoryAllocatorCreateInfo, StandardMemoryAllocator};
use vulkano::swapchain::{Surface, SurfaceApi};

static VULKAN_INSTANCE: OnceLock<VulkanInstanceData> = OnceLock::new();
const VULKAN_VERSION: Version = Version::major_minor(1, 3);
const DEVICE_EXTENSIONS: DeviceExtensions = DeviceExtensions {
    khr_swapchain: true,
    ..DeviceExtensions::empty()
};
const DEVICE_FEATURES: DeviceFeatures = DeviceFeatures {
    dynamic_rendering: true,
    ..DeviceFeatures::empty()
};

struct VulkanInstanceData {
    #[expect(unused)]
    library: Arc<VulkanLibrary>,
    #[expect(unused)]
    instance: Arc<Instance>,
    compatible_devices: Vec<Arc<PhysicalDevice>>
}


pub struct Renderer {
    device: Arc<Device>,
    queue: Arc<Queue>,
    surface: Arc<Surface>,
    memory_allocator: Arc<GenericMemoryAllocator<FreeListAllocator>>,
    command_buffer_allocator: Arc<StandardCommandBufferAllocator>
}

impl Renderer {
    pub unsafe fn attach_to_sdl(window: *mut SDL_Window) -> Result<Self, String> {
        let instance_data = VULKAN_INSTANCE.get().expect("vulkan not loaded");
        let mut device_found = None;
        let mut index_found = None;

        for device in &instance_data.compatible_devices {
            for (index, properties) in device.queue_family_properties().iter().enumerate() {
                if !properties.queue_flags.intersects(QueueFlags::GRAPHICS) {
                    continue;
                }
                unsafe {
                    if SDL_Vulkan_GetPresentationSupport(
                        transmute(instance_data.instance.as_ref().handle()),
                        transmute(device.handle()),
                        index as u32
                    ) {
                        index_found = Some(index);
                    }
                }
            }

            if index_found.is_some() {
                device_found = Some(device.clone());
                break;
            }
        }

        let Some(index_found) = index_found else {
            return Err("No compatible device found for drawing to windows...".to_string());
        };

        let device_found = device_found.expect("if we found an index, we found a device");

        let (device, mut queue) = match Device::new(device_found.clone(), DeviceCreateInfo {
            enabled_extensions: DEVICE_EXTENSIONS,
            enabled_features: DEVICE_FEATURES,
            queue_create_infos: vec![
                QueueCreateInfo {
                    queue_family_index: index_found as u32,
                    ..QueueCreateInfo::default()
                }
            ],
            ..DeviceCreateInfo::default()
        }) {
            Ok(n) => n,
            Err(e) => {
                return Err(format!("Failed to load renderer: {e:?}"))
            }
        };

        let queue = queue.next().expect("no queue");

        let memory_allocator = Arc::new(StandardMemoryAllocator::new_default(device.clone()));

        let command_buffer_allocator = Arc::new(StandardCommandBufferAllocator::new(
            device.clone(), Default::default(),
        ));
        let mut surface_raw = zeroed();

        if unsafe { !SDL_Vulkan_CreateSurface(
            window,
            transmute(instance_data.instance.handle()),
            null(),
            &mut surface_raw
        ) } {
            return Err("Failed to create SDL Vulkan surface".to_string());
        }

        let surface = unsafe { Arc::new(Surface::from_handle(
            instance_data.instance.clone(),
            transmute(surface_raw),
            // FIXME: this is wrong and might blow up
            SurfaceApi::DisplayPlane,
            None
        )) };

        Ok(Self {
            device,
            queue,
            surface,
            memory_allocator,
            command_buffer_allocator
        })
    }
}

pub fn preload_vulkan() -> Result<(), String> {
    if VULKAN_INSTANCE.get().is_some() {
        return Ok(())
    }

    let library = VulkanLibrary::new().map_err(|e| e.to_string())?;

    let success = unsafe { SDL_Vulkan_LoadLibrary(null()) };

    if !success {
        return Err("SDL failed to load Vulkan?".to_string());
    }

    let instance = setup_instance(&library, VULKAN_VERSION)?;

    let mut compatible_devices: Vec<Arc<PhysicalDevice>> = match instance.enumerate_physical_devices() {
        Ok(n) => n,
        Err(e) => return Err(format!("Failed to query devices: {e}"))
    }.filter(|e| {
        if e.api_version() < VULKAN_VERSION {
            return false
        }

        if !e.supported_extensions().contains(&DEVICE_EXTENSIONS) {
            return false
        }

        if !e.queue_family_properties().iter().any(|i| i.queue_flags.intersects(QueueFlags::GRAPHICS)) {
            return false
        }

        true
    }).collect();

    compatible_devices.sort_by_key(|i| match i.properties().device_type {
        PhysicalDeviceType::DiscreteGpu => 0,
        PhysicalDeviceType::IntegratedGpu => 1,
        PhysicalDeviceType::VirtualGpu => 2,
        PhysicalDeviceType::Cpu => 3,
        PhysicalDeviceType::Other => 4,
        _ => 5
    });

    if compatible_devices.is_empty() {
        return Err("No compatible devices found".to_string());
    }

    if VULKAN_INSTANCE.set(VulkanInstanceData {
        library,
        instance,
        compatible_devices
    }).is_err() {
        panic!("Vulkan instance already setup... tried to set it up again???")
    }

    Ok(())
}

fn setup_instance(library: &Arc<VulkanLibrary>, vulkan_version: Version) -> Result<Arc<Instance>, String> {
    unsafe {
        let mut count = 0;
        let extensions = SDL_Vulkan_GetInstanceExtensions(&mut count);
        let extensions = core::slice::from_raw_parts(extensions, count as usize)
            .iter()
            .map(|i| CStr::from_ptr(*i as *const c_char).to_str().unwrap());

        let extensions = InstanceExtensions::from_iter(extensions);
        if extensions.count() != count as u64 {
            return Err("Cannot load SDL + Vulkan because some extensions were unrecognized".to_string());
        }

        let mut extensions_to_enable = extensions.clone();
        let mut missing_extensions = extensions_to_enable.difference(&library.supported_extensions());

        // Not strictly necessary. SDL3 wants this enabled for some reason though...
        if missing_extensions.khr_portability_enumeration {
            missing_extensions.khr_portability_enumeration = false;
            extensions_to_enable.khr_portability_enumeration = false;
        }

        if library.api_version() < vulkan_version {
            return Err(format!("Insufficient Vulkan version {} (need {vulkan_version})", library.api_version()));
        }

        if !missing_extensions.is_empty() {
            return Err(format!("Missing extensions {missing_extensions:?}"))
        }

        let instance = match Instance::new(
            library.clone(),
            InstanceCreateInfo {
                max_api_version: Some(vulkan_version),
                enabled_extensions: extensions_to_enable,
                ..InstanceCreateInfo::default()
            }
        ) {
            Ok(n) => n,
            Err(e) => {
                return Err(format!("Failed to create instance: {e:?}"));
            }
        };

        Ok(instance)
    }
}

fn get_compatible_vulkan_devices() -> Vec<()> {
    let mut v = Vec::new();




    v
}
