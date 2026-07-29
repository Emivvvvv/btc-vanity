use std::borrow::Cow;
use wgpu::util::DeviceExt;

pub fn create_command_encoder(device: &wgpu::Device) -> wgpu::CommandEncoder {
    device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None })
}

pub fn create_sb_with_data(device: &wgpu::Device, data: &[u32]) -> wgpu::Buffer {
    device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: None,
        contents: bytemuck::cast_slice(data),
        usage: wgpu::BufferUsages::STORAGE
            | wgpu::BufferUsages::COPY_DST
            | wgpu::BufferUsages::COPY_SRC,
    })
}

pub fn create_ub_with_data(device: &wgpu::Device, data: &[u32]) -> wgpu::Buffer {
    device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: None,
        contents: bytemuck::cast_slice(data),
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
    })
}

pub fn create_empty_sb(device: &wgpu::Device, size: u64) -> wgpu::Buffer {
    device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size,
        mapped_at_creation: false,
        usage: wgpu::BufferUsages::STORAGE
            | wgpu::BufferUsages::COPY_DST
            | wgpu::BufferUsages::COPY_SRC,
    })
}

pub fn create_compute_pipeline(
    device: &wgpu::Device,
    code: &str,
    entry_point: &str,
) -> wgpu::ComputePipeline {
    let cs_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: None,
        source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(code)),
    });

    device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: None,
        layout: None,
        module: &cs_module,
        entry_point,
    })
}

pub fn execute_pipeline(
    command_encoder: &mut wgpu::CommandEncoder,
    compute_pipeline: &wgpu::ComputePipeline,
    bind_group: &wgpu::BindGroup,
    num_x_workgroups: u32,
    num_y_workgroups: u32,
    num_z_workgroups: u32,
) {
    let mut cpass = command_encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
        label: None,
        timestamp_writes: None,
    });
    cpass.set_pipeline(compute_pipeline);
    cpass.set_bind_group(0, bind_group, &[]);
    #[cfg(debug_assertions)]
    cpass.insert_debug_marker("compute pass");
    cpass.dispatch_workgroups(num_x_workgroups, num_y_workgroups, num_z_workgroups);
}

pub fn create_bind_group(
    device: &wgpu::Device,
    compute_pipeline: &wgpu::ComputePipeline,
    binding_idx: u32,
    buffers: &[&wgpu::Buffer],
) -> wgpu::BindGroup {
    let entries: Vec<wgpu::BindGroupEntry> = buffers
        .iter()
        .enumerate()
        .map(|(i, buf)| wgpu::BindGroupEntry {
            binding: i as u32,
            resource: buf.as_entire_binding(),
        })
        .collect();

    let bind_group_layout = compute_pipeline.get_bind_group_layout(binding_idx);
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: None,
        layout: &bind_group_layout,
        entries: &entries,
    })
}
