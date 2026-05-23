use anyhow::Result;

#[derive(Clone)]
pub struct GpuContext {
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub instance: wgpu::Instance,
    pub adapter: wgpu::Adapter,
}

impl GpuContext {
    pub async fn new() -> Result<Self> {
        let backends = if cfg!(target_os = "windows") {
            wgpu::Backends::from_env().unwrap_or_default()
        } else if cfg!(target_os = "macos") {
            wgpu::Backends::from_env().unwrap_or(wgpu::Backends::METAL | wgpu::Backends::VULKAN)
        } else {
            wgpu::Backends::from_env().unwrap_or(wgpu::Backends::VULKAN)
        };

        // WGPU_BACKEND=vulkan,dx12 (comma separated)
        // let backends = wgpu::Backends::from_env().unwrap_or_default();

        let flags = wgpu::InstanceFlags::from_build_config().with_env();
        let memory_budget_thresholds = wgpu::MemoryBudgetThresholds::default();
        let backend_options = wgpu::BackendOptions::from_env_or_default();
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends,
            flags,
            memory_budget_thresholds,
            backend_options,
        });

        let adapter = wgpu::util::initialize_adapter_from_env_or_default(&instance, None).await?;
        // let info = adapter.get_info();
        // println!("{:?} - {:?}", info.name, info.backend);
        let features = adapter.features();
        let limits = wgpu::Limits::default();
        let maybe_features = wgpu::Features::CLEAR_TEXTURE | wgpu::Features::PIPELINE_CACHE;

        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: None,
                required_features: features & maybe_features,
                required_limits: limits,
                ..Default::default()
            })
            .await?;

        Ok(Self {
            device,
            queue,
            instance,
            adapter,
        })
    }
}
