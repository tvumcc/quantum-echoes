use vulkano::*;
use vulkano::swapchain::*;
use vulkano::instance::*;
use vulkano::device::*;
use vulkano::device::physical::*;
use vulkano::memory::allocator::*;
use vulkano::command_buffer::allocator::*;
use vulkano::descriptor_set::allocator::*;

use winit::window::*;
use winit::event_loop::*;

use std::sync::Arc;

pub struct App {
    pub physical_device: Arc<PhysicalDevice>,
    pub device: Arc<Device>,
    pub queue: Arc<Queue>,

    pub window: Arc<Window>,    
    pub surface: Arc<Surface>,

    pub memory_allocator: Arc<StandardMemoryAllocator>,
    pub descriptor_set_allocator: Arc<StandardDescriptorSetAllocator>,
    pub command_buffer_allocator: StandardCommandBufferAllocator
}

impl App {
    pub fn new(event_loop: &EventLoop<()>, window_title: &str) -> Self {
        let required_extensions = Surface::required_extensions(&event_loop);
        let device_extensions = DeviceExtensions {
            khr_swapchain: true,
            ..DeviceExtensions::empty()
        }; 

        let library = VulkanLibrary::new().expect("No local Vulkan library/DLL. Cannot initialize app.");
        let instance = Instance::new(
            library,
            InstanceCreateInfo {
                flags: InstanceCreateFlags::ENUMERATE_PORTABILITY,
                enabled_extensions: required_extensions,
                ..Default::default()
            }
        ).expect("Failed to create instance");

        let window = Arc::new(WindowBuilder::new().build(&event_loop).unwrap());
        let surface = Surface::from_window(instance.clone(), window.clone()).unwrap();

        // window.set_visible(false);
        window.set_title(window_title);

        // Create PhysicalDevice, Device, and Queue
        let physical_device = instance.enumerate_physical_devices().unwrap().next().unwrap();
        let queue_family_index = physical_device
            .queue_family_properties()
            .iter()
            .position(|queue_family_properties| {
                queue_family_properties.queue_flags.contains(QueueFlags::GRAPHICS)
            })
            .expect("couldn't find a graphical queue family") as u32;

        let (device, mut queues) = Device::new(
            physical_device.clone(),
            DeviceCreateInfo {
                queue_create_infos: vec![QueueCreateInfo {
                    queue_family_index,
                    ..Default::default()
                }],
                enabled_extensions: device_extensions,
                ..Default::default()
            }        
        )
        .expect("failed to create device");
        let queue = queues.next().unwrap();

        // Create Allocators
        let memory_allocator = Arc::new(StandardMemoryAllocator::new_default(device.clone()));
        let descriptor_set_allocator = Arc::new(StandardDescriptorSetAllocator::new(
            device.clone(),
            StandardDescriptorSetAllocatorCreateInfo::default()
        )); 
        let command_buffer_allocator = StandardCommandBufferAllocator::new(
            device.clone(),
            StandardCommandBufferAllocatorCreateInfo::default()
        ); 

        App {
            physical_device: physical_device,
            device: device,
            queue: queue,

            window: window,
            surface: surface,

            memory_allocator: memory_allocator,
            descriptor_set_allocator: descriptor_set_allocator,
            command_buffer_allocator: command_buffer_allocator
        }
    }
}
