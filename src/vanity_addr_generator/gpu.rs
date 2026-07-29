use crate::error::VanityError;
#[cfg(feature = "gpu")]
use crate::keys_and_address::BitcoinKeyPair;

use crate::vanity_addr_generator::chain::VanityChain;
use crate::vanity_addr_generator::vanity_addr::{GpuSearchTarget, VanityMode};

use crate::wgpu_sig_ops::arithmetic::{biguint_to_limbs_le, calc_num_limbs};
use crate::wgpu_sig_ops::gpu::{
    create_bind_group, create_command_encoder, create_compute_pipeline, create_empty_sb,
    create_sb_with_data, create_ub_with_data, execute_pipeline,
};
use crate::wgpu_sig_ops::precompute::{ed25519_bases, secp256k1_bases};
use crate::wgpu_sig_ops::shader::{render_ed25519_search_shader, render_secp256k1_search_shader};
use bitcoin::hashes::{ripemd160, sha256, Hash};
use bitcoin::secp256k1::{rand, PublicKey as SecpPublicKey, Secp256k1, SecretKey};
use bitcoin::{Address, Network, PublicKey};
use num_bigint::BigUint;
use pollster::block_on;
use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

pub const GPU_BATCH_SIZE: usize = 262_144;
pub const GPU_MAX_BATCH_SIZE: usize = 2_097_152;
pub const GPU_RING_DEPTH: usize = 4;

const LOG_LIMB_SIZE: u32 = 13;
const WORKGROUP_SIZE: u32 = 256;
const PATTERN_CAPACITY: usize = 40;
const RESULT_WORDS: usize = 64;
const THROTTLED_GPU_BATCH_SIZE: usize = 4_096;

const RESULT_WINNER_INDEX: usize = 0;
const RESULT_ATTEMPTS_LO_INDEX: usize = 1;
const RESULT_ATTEMPTS_HI_INDEX: usize = 2;
const RESULT_BATCHES_INDEX: usize = 3;
const RESULT_ADDRESS_LEN_INDEX: usize = 4;
const RESULT_SCALAR_INDEX: usize = 5;
const RESULT_DEBUG_HASH_INDEX: usize = 13;
const RESULT_ADDRESS_INDEX: usize = 21;

#[derive(Clone, Debug)]
pub struct GpuMatch {
    pub private_key_bytes: [u8; 32],
    pub address: String,
    pub attempts: u64,
    pub batches: u64,
    pub debug_hash: Option<Vec<u8>>,
}

#[derive(Clone, Debug)]
pub struct GpuBitcoinDebug {
    pub private_key_bytes: [u8; 32],
    pub address: String,
    pub compressed_public_key: [u8; 33],
    pub sha256: [u8; 32],
    pub hash160: [u8; 20],
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct GpuTuning {
    pub batch_size: usize,
    pub workgroup_size: u32,
    pub candidates_per_invocation: u32,
    pub ring_depth: usize,
}

impl Default for GpuTuning {
    fn default() -> Self {
        Self {
            batch_size: GPU_BATCH_SIZE,
            workgroup_size: WORKGROUP_SIZE,
            candidates_per_invocation: 16,
            ring_depth: 2,
        }
    }
}

impl GpuTuning {
    pub fn attempts_for_batches(self, batches: usize) -> u64 {
        let batch_size = u64::try_from(self.batch_size).unwrap_or(u64::MAX);
        let batch_count = u64::try_from(batches).unwrap_or(u64::MAX);
        batch_size.saturating_mul(batch_count)
    }
}

#[derive(Copy, Clone, Debug)]
pub(crate) struct GpuDutyCycle {
    limit: u8,
}

impl GpuDutyCycle {
    pub(crate) fn new(limit: u8) -> Result<Self, VanityError> {
        if !(1..=100).contains(&limit) {
            return Err(VanityError::InvalidGpuUsageLimit);
        }
        Ok(Self { limit })
    }

    pub(crate) fn idle_for(active: Duration, limit: u8) -> Duration {
        if limit >= 100 {
            return Duration::ZERO;
        }
        active.mul_f64(f64::from(100 - limit) / f64::from(limit))
    }

    pub(crate) fn responsive_tuning(mut tuning: GpuTuning, limit: u8) -> GpuTuning {
        if limit < 100 {
            tuning.batch_size = tuning.batch_size.min(THROTTLED_GPU_BATCH_SIZE);
            tuning.ring_depth = 1;
        }
        tuning
    }

    fn throttle(self, active: Duration) {
        let idle = Self::idle_for(active, self.limit);
        if !idle.is_zero() {
            thread::sleep(idle);
        }
    }
}

struct DispatchSlot {
    _counter_buf: wgpu::Buffer,
    params_buf: wgpu::Buffer,
    result_buf: wgpu::Buffer,
    status_readback_buf: wgpu::Buffer,
    result_readback_buf: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
    readback_state: Mutex<SlotReadbackState>,
}

enum SlotReadbackState {
    Idle,
    PendingStatus(Receiver<Result<(), wgpu::BufferAsyncError>>),
    PendingResult(Receiver<Result<(), wgpu::BufferAsyncError>>),
}

enum SlotInspection {
    Pending,
    Ready(Option<GpuMatch>),
}

struct ExactSearchPipeline {
    compute_pipeline: wgpu::ComputePipeline,
    seed_buf: wgpu::Buffer,
    pattern_buf: wgpu::Buffer,
    slots: Vec<DispatchSlot>,
    execution_lock: Arc<Mutex<()>>,
}

pub struct Secp256k1GpuEngine {
    device: wgpu::Device,
    queue: wgpu::Queue,
    adapter_info: wgpu::AdapterInfo,
    ethereum_pipeline: ExactSearchPipeline,
    bitcoin_pipeline: ExactSearchPipeline,
    solana_pipeline: ExactSearchPipeline,
    num_limbs: usize,
}

async fn request_supported_adapter(
    instance: &wgpu::Instance,
    min_storage_buffers_per_shader_stage: u32,
) -> Option<wgpu::Adapter> {
    for force_fallback_adapter in [false, true] {
        for power_preference in [
            wgpu::PowerPreference::HighPerformance,
            wgpu::PowerPreference::LowPower,
        ] {
            if let Some(adapter) = instance
                .request_adapter(&wgpu::RequestAdapterOptions {
                    power_preference,
                    force_fallback_adapter,
                    compatible_surface: None,
                })
                .await
            {
                if adapter.limits().max_storage_buffers_per_shader_stage
                    >= min_storage_buffers_per_shader_stage
                {
                    return Some(adapter);
                }
            }
        }
    }

    None
}

impl Secp256k1GpuEngine {
    pub fn new() -> Result<Self, VanityError> {
        const MIN_STORAGE_BUFFERS_PER_SHADER_STAGE: u32 = 5;
        let (device, queue, adapter_info) = block_on(async {
            let instance = wgpu::Instance::default();
            let adapter =
                request_supported_adapter(&instance, MIN_STORAGE_BUFFERS_PER_SHADER_STAGE)
                    .await
                    .ok_or(VanityError::GpuAdapterUnavailable)?;

            let mut required_limits = wgpu::Limits::downlevel_defaults();
            required_limits.max_storage_buffers_per_shader_stage =
                MIN_STORAGE_BUFFERS_PER_SHADER_STAGE;

            let adapter_info = adapter.get_info();
            let (device, queue) = adapter
                .request_device(
                    &wgpu::DeviceDescriptor {
                        label: Some("btc-vanity-gpu-device"),
                        required_features: wgpu::Features::empty(),
                        required_limits,
                    },
                    None,
                )
                .await
                .map_err(|_| VanityError::GpuAdapterUnavailable)?;

            Ok::<_, VanityError>((device, queue, adapter_info))
        })?;

        let num_limbs = calc_num_limbs(LOG_LIMB_SIZE, 256);
        let secp_table_limbs = secp256k1_bases(LOG_LIMB_SIZE);
        let secp_table_buf = create_sb_with_data(&device, &secp_table_limbs);
        let ed25519_table_limbs = ed25519_bases(LOG_LIMB_SIZE);
        let ed25519_table_buf = create_sb_with_data(&device, &ed25519_table_limbs);
        let execution_lock = Arc::new(Mutex::new(()));

        let ethereum_pipeline = Self::create_exact_pipeline(
            &device,
            &secp_table_buf,
            num_limbs,
            "secp256k1_eth_vanity_search.wgsl",
            "secp256k1_eth_vanity_search",
            false,
            Arc::clone(&execution_lock),
        );
        let bitcoin_pipeline = Self::create_exact_pipeline(
            &device,
            &secp_table_buf,
            num_limbs,
            "secp256k1_btc_vanity_search.wgsl",
            "secp256k1_btc_vanity_search",
            false,
            Arc::clone(&execution_lock),
        );
        let solana_pipeline = Self::create_exact_pipeline(
            &device,
            &ed25519_table_buf,
            num_limbs,
            "ed25519_sol_vanity_search.wgsl",
            "ed25519_sol_vanity_search",
            true,
            Arc::clone(&execution_lock),
        );

        Ok(Self {
            device,
            queue,
            adapter_info,
            ethereum_pipeline,
            bitcoin_pipeline,
            solana_pipeline,
            num_limbs,
        })
    }

    fn create_exact_pipeline(
        device: &wgpu::Device,
        table_buf: &wgpu::Buffer,
        num_limbs: usize,
        shader_name: &str,
        entry_point: &str,
        is_ed25519: bool,
        execution_lock: Arc<Mutex<()>>,
    ) -> ExactSearchPipeline {
        let seed_buf = create_empty_sb(device, (num_limbs * std::mem::size_of::<u32>()) as u64);
        let pattern_buf = create_empty_sb(
            device,
            (PATTERN_CAPACITY * std::mem::size_of::<u32>()) as u64,
        );
        let source = if is_ed25519 {
            render_ed25519_search_shader(shader_name, LOG_LIMB_SIZE)
        } else {
            render_secp256k1_search_shader(shader_name, LOG_LIMB_SIZE)
        };
        let compute_pipeline = create_compute_pipeline(device, &source, entry_point);
        let mut slots = Vec::with_capacity(GPU_RING_DEPTH);

        for slot_index in 0..GPU_RING_DEPTH {
            let counter_buf =
                create_empty_sb(device, (calc_num_limbs(LOG_LIMB_SIZE, 256) * 4) as u64);
            let params_buf = create_ub_with_data(device, &[0u32; 8]);
            let result_buf =
                create_empty_sb(device, (RESULT_WORDS * std::mem::size_of::<u32>()) as u64);
            let status_readback_buf = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some(&format!("btc-vanity-status-readback-{slot_index}")),
                size: std::mem::size_of::<u32>() as u64,
                usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            let result_readback_buf = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some(&format!("btc-vanity-result-readback-{slot_index}")),
                size: (RESULT_WORDS * std::mem::size_of::<u32>()) as u64,
                usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            let bind_group = create_bind_group(
                device,
                &compute_pipeline,
                0,
                &[
                    table_buf,
                    &seed_buf,
                    &counter_buf,
                    &pattern_buf,
                    &result_buf,
                    &params_buf,
                ],
            );

            slots.push(DispatchSlot {
                _counter_buf: counter_buf,
                params_buf,
                result_buf,
                status_readback_buf,
                result_readback_buf,
                bind_group,
                readback_state: Mutex::new(SlotReadbackState::Idle),
            });
        }

        ExactSearchPipeline {
            compute_pipeline,
            seed_buf,
            pattern_buf,
            slots,
            execution_lock,
        }
    }

    pub fn adapter_name(&self) -> String {
        format!(
            "{:?}: {}",
            self.adapter_info.backend, self.adapter_info.name
        )
    }

    pub fn generate_private_keys(&self, count: usize) -> Vec<[u8; 32]> {
        let mut rng = rand::thread_rng();
        let mut private_keys = Vec::with_capacity(count);

        for _ in 0..count {
            private_keys.push(SecretKey::new(&mut rng).secret_bytes());
        }

        private_keys
    }

    pub fn derive_public_keys(
        &self,
        private_keys: &[[u8; 32]],
    ) -> Result<Vec<[u8; 65]>, VanityError> {
        if private_keys.is_empty() {
            return Ok(Vec::new());
        }

        let mut public_keys = Vec::with_capacity(private_keys.len());
        let secp = Secp256k1::new();

        for private_key in private_keys {
            let secret_key = SecretKey::from_slice(private_key)
                .map_err(|_| VanityError::GpuInvalidResult("invalid private key"))?;
            let public_key = SecpPublicKey::from_secret_key(&secp, &secret_key);
            public_keys.push(public_key.serialize_uncompressed());
        }

        Ok(public_keys)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn run_exact_batches(
        &self,
        target: GpuSearchTarget,
        seed: [u8; 32],
        pattern: &[u8],
        case_sensitive: bool,
        vanity_mode: VanityMode,
        batch_size: usize,
        batches: usize,
    ) -> Result<Option<GpuMatch>, VanityError> {
        let tuning = GpuTuning {
            batch_size,
            ..GpuTuning::default()
        };
        self.run_exact_batches_with_tuning(
            target,
            seed,
            pattern,
            case_sensitive,
            vanity_mode,
            tuning,
            batches,
        )
    }

    pub fn search_exact(
        &self,
        target: GpuSearchTarget,
        seed: [u8; 32],
        pattern: &[u8],
        case_sensitive: bool,
        vanity_mode: VanityMode,
        batch_size: usize,
    ) -> Result<GpuMatch, VanityError> {
        let tuning = GpuTuning {
            batch_size,
            ..GpuTuning::default()
        };
        self.search_exact_with_tuning(
            target,
            seed,
            pattern,
            case_sensitive,
            vanity_mode,
            tuning,
            100,
            None,
        )
        .map(|opt| opt.expect("pure GPU search should not return None without stop flag"))
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn run_exact_batches_with_tuning(
        &self,
        target: GpuSearchTarget,
        seed: [u8; 32],
        pattern: &[u8],
        case_sensitive: bool,
        vanity_mode: VanityMode,
        tuning: GpuTuning,
        batches: usize,
    ) -> Result<Option<GpuMatch>, VanityError> {
        self.validate_exact_request(pattern, vanity_mode, tuning)?;
        let pipeline = self.pipeline_for_target(target)?;
        let _execution_guard = match pipeline.execution_lock.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };
        self.write_seed(&pipeline.seed_buf, &seed);
        self.write_pattern(&pipeline.pattern_buf, pattern, case_sensitive);
        let pattern_len = pattern.len();

        if batches == 0 {
            return Ok(None);
        }

        let ring_depth = tuning.ring_depth.min(pipeline.slots.len()).max(1);
        let mut inflight: VecDeque<(usize, usize)> = VecDeque::with_capacity(ring_depth);
        let mut next_batch_index = 0usize;
        let mut pending_streak = 0usize;

        while next_batch_index < batches && inflight.len() < ring_depth {
            let batch_index = next_batch_index;
            let slot_index = batch_index % ring_depth;
            self.submit_exact_slot(
                pipeline,
                slot_index,
                pattern_len,
                case_sensitive,
                vanity_mode,
                tuning,
                batch_index as u32,
            );
            inflight.push_back((slot_index, batch_index));
            next_batch_index += 1;
        }

        while let Some((slot_index, batch_index)) = inflight.pop_front() {
            match self.inspect_slot(pipeline, slot_index)? {
                SlotInspection::Pending => {
                    pending_streak += 1;
                    inflight.push_back((slot_index, batch_index));
                }
                SlotInspection::Ready(Some(found)) => {
                    if !self.is_valid_match_for_target(target, &found) {
                        if next_batch_index < batches {
                            let new_batch_index = next_batch_index;
                            self.submit_exact_slot(
                                pipeline,
                                slot_index,
                                pattern_len,
                                case_sensitive,
                                vanity_mode,
                                tuning,
                                new_batch_index as u32,
                            );
                            inflight.push_back((slot_index, new_batch_index));
                            next_batch_index += 1;
                        }
                        continue;
                    }
                    self.cleanup_inflight_readbacks(pipeline, &inflight);
                    return Ok(Some(found));
                }
                SlotInspection::Ready(None) => {
                    pending_streak = 0;

                    if next_batch_index < batches {
                        let new_batch_index = next_batch_index;
                        self.submit_exact_slot(
                            pipeline,
                            slot_index,
                            pattern_len,
                            case_sensitive,
                            vanity_mode,
                            tuning,
                            new_batch_index as u32,
                        );
                        inflight.push_back((slot_index, new_batch_index));
                        next_batch_index += 1;
                    }
                }
            }

            if pending_streak >= inflight.len().max(1) {
                self.wait_for_gpu_progress();
                pending_streak = 0;
            }
        }

        Ok(None)
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn search_exact_with_tuning(
        &self,
        target: GpuSearchTarget,
        seed: [u8; 32],
        pattern: &[u8],
        case_sensitive: bool,
        vanity_mode: VanityMode,
        tuning: GpuTuning,
        gpu_usage_limit: u8,
        stop: Option<&Arc<AtomicBool>>,
    ) -> Result<Option<GpuMatch>, VanityError> {
        let tuning = GpuDutyCycle::responsive_tuning(tuning, gpu_usage_limit);
        self.validate_exact_request(pattern, vanity_mode, tuning)?;
        let duty_cycle = GpuDutyCycle::new(gpu_usage_limit)?;
        let pipeline = self.pipeline_for_target(target)?;
        let _execution_guard = match pipeline.execution_lock.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };
        self.write_seed(&pipeline.seed_buf, &seed);
        self.write_pattern(&pipeline.pattern_buf, pattern, case_sensitive);
        let pattern_len = pattern.len();

        let ring_depth = tuning.ring_depth.min(pipeline.slots.len()).max(1);
        let mut inflight: VecDeque<(usize, usize)> = VecDeque::with_capacity(ring_depth);
        let mut next_batch_index = 0usize;

        loop {
            if let Some(stop_flag) = stop {
                if stop_flag.load(Ordering::Relaxed) {
                    return Ok(None);
                }
            }

            let active_started = Instant::now();
            while inflight.len() < ring_depth {
                let batch_index = next_batch_index;
                let slot_index = batch_index % ring_depth;
                self.submit_exact_slot(
                    pipeline,
                    slot_index,
                    pattern_len,
                    case_sensitive,
                    vanity_mode,
                    tuning,
                    batch_index as u32,
                );
                inflight.push_back((slot_index, batch_index));
                next_batch_index += 1;
            }

            let mut pending_streak = 0usize;
            while let Some((slot_index, batch_index)) = inflight.pop_front() {
                if let Some(stop_flag) = stop {
                    if stop_flag.load(Ordering::Relaxed) {
                        inflight.push_front((slot_index, batch_index));
                        self.cleanup_inflight_readbacks(pipeline, &inflight);
                        return Ok(None);
                    }
                }

                match self.inspect_slot(pipeline, slot_index)? {
                    SlotInspection::Pending => {
                        pending_streak += 1;
                        inflight.push_back((slot_index, batch_index));
                    }
                    SlotInspection::Ready(Some(found)) => {
                        pending_streak = 0;
                        if self.is_valid_match_for_target(target, &found) {
                            self.cleanup_inflight_readbacks(pipeline, &inflight);
                            return Ok(Some(found));
                        }
                    }
                    SlotInspection::Ready(None) => {
                        pending_streak = 0;
                    }
                }

                if pending_streak >= inflight.len().max(1) {
                    self.wait_for_gpu_progress();
                    pending_streak = 0;
                }
            }

            duty_cycle.throttle(active_started.elapsed());
        }
    }

    pub fn debug_bitcoin_candidate(
        &self,
        seed: [u8; 32],
    ) -> Result<Option<GpuBitcoinDebug>, VanityError> {
        let keypair = <BitcoinKeyPair as VanityChain>::from_private_key_bytes(seed)?;
        let secp_public_key = keypair.get_public_key().inner;
        let compressed_public_key = secp_public_key.serialize();
        let sha256 = sha256::Hash::hash(&compressed_public_key);
        let hash160 = ripemd160::Hash::hash(sha256.as_byte_array());
        let address = Address::p2pkh(PublicKey::new(secp_public_key), Network::Bitcoin).to_string();

        Ok(Some(GpuBitcoinDebug {
            private_key_bytes: seed,
            address,
            compressed_public_key,
            sha256: *sha256.as_byte_array(),
            hash160: *hash160.as_byte_array(),
        }))
    }

    fn validate_exact_request(
        &self,
        pattern: &[u8],
        vanity_mode: VanityMode,
        tuning: GpuTuning,
    ) -> Result<(), VanityError> {
        if tuning.batch_size == 0 || tuning.batch_size > GPU_MAX_BATCH_SIZE {
            return Err(VanityError::GpuInvalidResult("invalid GPU batch size"));
        }
        if pattern.len() > PATTERN_CAPACITY {
            return Err(VanityError::GpuInvalidResult(
                "pattern is too long for GPU search",
            ));
        }
        if matches!(vanity_mode, VanityMode::Regex) {
            return Err(VanityError::GpuRegexUnsupported);
        }
        if tuning.candidates_per_invocation == 0 {
            return Err(VanityError::GpuInvalidResult(
                "GPU candidates_per_invocation must be non-zero",
            ));
        }

        Ok(())
    }

    fn pipeline_for_target(
        &self,
        target: GpuSearchTarget,
    ) -> Result<&ExactSearchPipeline, VanityError> {
        match target {
            GpuSearchTarget::Bitcoin => Ok(&self.bitcoin_pipeline),
            GpuSearchTarget::Ethereum => Ok(&self.ethereum_pipeline),
            GpuSearchTarget::Solana => Ok(&self.solana_pipeline),
        }
    }

    fn write_seed(&self, seed_buf: &wgpu::Buffer, seed: &[u8; 32]) {
        let scalar = BigUint::from_bytes_be(seed);
        let seed_limbs = biguint_to_limbs_le(&scalar, self.num_limbs, LOG_LIMB_SIZE);
        self.queue
            .write_buffer(seed_buf, 0, bytemuck::cast_slice(&seed_limbs));
    }

    fn write_pattern(&self, pattern_buf: &wgpu::Buffer, pattern: &[u8], case_sensitive: bool) {
        let mut words = [0u32; PATTERN_CAPACITY];
        for (index, byte) in pattern.iter().enumerate() {
            words[index] = if case_sensitive {
                *byte as u32
            } else {
                byte.to_ascii_lowercase() as u32
            };
        }
        self.queue
            .write_buffer(pattern_buf, 0, bytemuck::cast_slice(&words));
    }

    #[allow(clippy::too_many_arguments)]
    fn write_params(
        &self,
        params_buf: &wgpu::Buffer,
        batch_size: usize,
        vanity_mode: VanityMode,
        pattern_len: usize,
        case_sensitive: bool,
        attempt_offset: u64,
        batch_index: u32,
        candidates_per_invocation: u32,
    ) {
        let params = [
            batch_size as u32,
            vanity_mode_to_gpu(vanity_mode),
            pattern_len as u32,
            case_sensitive as u32,
            attempt_offset as u32,
            (attempt_offset >> 32) as u32,
            batch_index,
            candidates_per_invocation,
        ];
        self.queue
            .write_buffer(params_buf, 0, bytemuck::cast_slice(&params));
    }

    fn reset_result_buffer(&self, result_buf: &wgpu::Buffer) {
        let sentinel = u32::MAX;
        self.queue
            .write_buffer(result_buf, 0, bytemuck::bytes_of(&sentinel));
    }

    #[allow(clippy::too_many_arguments)]
    fn submit_exact_slot(
        &self,
        pipeline: &ExactSearchPipeline,
        slot_index: usize,
        pattern_len: usize,
        case_sensitive: bool,
        vanity_mode: VanityMode,
        tuning: GpuTuning,
        batch_index: u32,
    ) {
        let attempt_offset = batch_index as u64 * tuning.batch_size as u64;
        let slot = &pipeline.slots[slot_index];
        self.write_params(
            &slot.params_buf,
            tuning.batch_size,
            vanity_mode,
            pattern_len,
            case_sensitive,
            attempt_offset,
            batch_index,
            tuning.candidates_per_invocation,
        );

        // Reset the result sentinel immediately before dispatch to avoid stale winners.
        self.reset_result_buffer(&slot.result_buf);

        let mut command_encoder = create_command_encoder(&self.device);
        execute_pipeline(
            &mut command_encoder,
            &pipeline.compute_pipeline,
            &slot.bind_group,
            dispatch_workgroups_for(
                tuning.batch_size,
                tuning.candidates_per_invocation,
                tuning.workgroup_size,
            ),
            1,
            1,
        );
        command_encoder.copy_buffer_to_buffer(
            &slot.result_buf,
            0,
            &slot.status_readback_buf,
            0,
            std::mem::size_of::<u32>() as u64,
        );
        self.queue.submit(Some(command_encoder.finish()));

        let buffer_slice = slot.status_readback_buf.slice(..);
        let (sender, receiver) = mpsc::channel();
        buffer_slice.map_async(wgpu::MapMode::Read, move |value| {
            let _ = sender.send(value);
        });

        let mut state = match slot.readback_state.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };
        *state = SlotReadbackState::PendingStatus(receiver);
    }

    fn inspect_slot(
        &self,
        pipeline: &ExactSearchPipeline,
        slot_index: usize,
    ) -> Result<SlotInspection, VanityError> {
        let slot = &pipeline.slots[slot_index];

        let mut state = match slot.readback_state.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };

        match &mut *state {
            SlotReadbackState::Idle => Ok(SlotInspection::Pending),
            SlotReadbackState::PendingStatus(receiver) => match receiver.try_recv() {
                Ok(Ok(())) => {
                    let status_bytes = slot.status_readback_buf.slice(..).get_mapped_range();
                    let status_words = bytemuck::cast_slice::<u8, u32>(&status_bytes);
                    if status_words.is_empty() {
                        drop(status_bytes);
                        slot.status_readback_buf.unmap();
                        *state = SlotReadbackState::Idle;
                        return Err(VanityError::GpuInvalidResult(
                            "GPU status buffer was smaller than expected",
                        ));
                    }

                    let winner = status_words[RESULT_WINNER_INDEX];
                    drop(status_bytes);
                    slot.status_readback_buf.unmap();

                    if winner == u32::MAX {
                        *state = SlotReadbackState::Idle;
                        return Ok(SlotInspection::Ready(None));
                    }

                    let mut command_encoder = create_command_encoder(&self.device);
                    command_encoder.copy_buffer_to_buffer(
                        &slot.result_buf,
                        0,
                        &slot.result_readback_buf,
                        0,
                        slot.result_readback_buf.size(),
                    );
                    self.queue.submit(Some(command_encoder.finish()));

                    let buffer_slice = slot.result_readback_buf.slice(..);
                    let (sender, receiver) = mpsc::channel();
                    buffer_slice.map_async(wgpu::MapMode::Read, move |value| {
                        let _ = sender.send(value);
                    });
                    *state = SlotReadbackState::PendingResult(receiver);
                    Ok(SlotInspection::Pending)
                }
                Ok(Err(_)) => {
                    *state = SlotReadbackState::Idle;
                    Err(VanityError::GpuInvalidResult(
                        "failed to read GPU status buffer",
                    ))
                }
                Err(TryRecvError::Empty) => Ok(SlotInspection::Pending),
                Err(TryRecvError::Disconnected) => {
                    *state = SlotReadbackState::Idle;
                    Err(VanityError::GpuInvalidResult(
                        "GPU status readback channel disconnected",
                    ))
                }
            },
            SlotReadbackState::PendingResult(receiver) => match receiver.try_recv() {
                Ok(Ok(())) => {
                    let bytes = slot.result_readback_buf.slice(..).get_mapped_range();
                    let words = bytemuck::cast_slice::<u8, u32>(&bytes);
                    if words.len() < RESULT_WORDS {
                        drop(bytes);
                        slot.result_readback_buf.unmap();
                        *state = SlotReadbackState::Idle;
                        return Err(VanityError::GpuInvalidResult(
                            "GPU result buffer was smaller than expected",
                        ));
                    }

                    let mut result = [0u32; RESULT_WORDS];
                    result.copy_from_slice(&words[..RESULT_WORDS]);
                    drop(bytes);
                    slot.result_readback_buf.unmap();
                    *state = SlotReadbackState::Idle;
                    Ok(SlotInspection::Ready(parse_match_result(&result)))
                }
                Ok(Err(_)) => {
                    *state = SlotReadbackState::Idle;
                    Err(VanityError::GpuInvalidResult(
                        "failed to read GPU result buffer",
                    ))
                }
                Err(TryRecvError::Empty) => Ok(SlotInspection::Pending),
                Err(TryRecvError::Disconnected) => {
                    *state = SlotReadbackState::Idle;
                    Err(VanityError::GpuInvalidResult(
                        "GPU result readback channel disconnected",
                    ))
                }
            },
        }
    }

    fn wait_for_gpu_progress(&self) {
        let _ = self.device.poll(wgpu::Maintain::Wait);
    }

    fn is_valid_match_for_target(&self, target: GpuSearchTarget, found: &GpuMatch) -> bool {
        if found.address.is_empty() || found.attempts == 0 || found.batches == 0 {
            return false;
        }

        match target {
            GpuSearchTarget::Bitcoin | GpuSearchTarget::Ethereum => {
                SecretKey::from_slice(&found.private_key_bytes).is_ok()
            }
            GpuSearchTarget::Solana => true,
        }
    }

    fn is_slot_readback_pending(&self, pipeline: &ExactSearchPipeline, slot_index: usize) -> bool {
        let slot = &pipeline.slots[slot_index];
        let state = match slot.readback_state.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };
        matches!(
            *state,
            SlotReadbackState::PendingStatus(_) | SlotReadbackState::PendingResult(_)
        )
    }

    fn cleanup_inflight_readbacks(
        &self,
        pipeline: &ExactSearchPipeline,
        inflight: &VecDeque<(usize, usize)>,
    ) {
        let mut seen = vec![false; pipeline.slots.len()];
        for (slot_index, _) in inflight {
            if *slot_index >= seen.len() || seen[*slot_index] {
                continue;
            }
            seen[*slot_index] = true;

            while self.is_slot_readback_pending(pipeline, *slot_index) {
                match self.inspect_slot(pipeline, *slot_index) {
                    Ok(SlotInspection::Pending) => self.wait_for_gpu_progress(),
                    Ok(SlotInspection::Ready(_)) => break,
                    Err(_) => break,
                }
            }
        }
    }
}

pub fn gpu_backend_available() -> bool {
    const MIN_STORAGE_BUFFERS_PER_SHADER_STAGE: u32 = 5;
    block_on(async {
        let instance = wgpu::Instance::default();
        request_supported_adapter(&instance, MIN_STORAGE_BUFFERS_PER_SHADER_STAGE)
            .await
            .is_some()
    })
}

fn vanity_mode_to_gpu(vanity_mode: VanityMode) -> u32 {
    match vanity_mode {
        VanityMode::Prefix => 0,
        VanityMode::Suffix => 1,
        VanityMode::Anywhere => 2,
        VanityMode::Regex => 3,
    }
}

fn parse_match_result(result: &[u32]) -> Option<GpuMatch> {
    if result.get(RESULT_WINNER_INDEX).copied().unwrap_or(u32::MAX) == u32::MAX {
        return None;
    }

    if result.len() < (RESULT_SCALAR_INDEX + 8)
        || result.len() < (RESULT_DEBUG_HASH_INDEX + 8)
        || result.len() <= RESULT_ADDRESS_LEN_INDEX
    {
        return None;
    }

    let mut private_key_bytes = [0u8; 32];
    for (index, word) in result[RESULT_SCALAR_INDEX..RESULT_SCALAR_INDEX + 8]
        .iter()
        .enumerate()
    {
        private_key_bytes[index * 4..(index + 1) * 4].copy_from_slice(&word.to_le_bytes());
    }

    let address_len = result.get(RESULT_ADDRESS_LEN_INDEX).copied()? as usize;
    if address_len > 44 {
        return None;
    }

    let address_words_needed = address_len.div_ceil(4);
    let address_end = RESULT_ADDRESS_INDEX.checked_add(address_words_needed)?;
    if address_end > result.len() {
        return None;
    }

    let address_bytes: Vec<u8> = result[RESULT_ADDRESS_INDEX..address_end]
        .iter()
        .flat_map(|w| w.to_le_bytes())
        .take(address_len)
        .collect();

    let address = String::from_utf8_lossy(&address_bytes).to_string();

    let mut debug_hash = Vec::new();
    for word in result[RESULT_DEBUG_HASH_INDEX..RESULT_DEBUG_HASH_INDEX + 8].iter() {
        debug_hash.extend_from_slice(&word.to_le_bytes());
    }
    let attempts =
        ((result[RESULT_ATTEMPTS_HI_INDEX] as u64) << 32) | result[RESULT_ATTEMPTS_LO_INDEX] as u64;
    let batches = result[RESULT_BATCHES_INDEX] as u64;

    Some(GpuMatch {
        private_key_bytes,
        address,
        attempts,
        batches,
        debug_hash: Some(debug_hash),
    })
}

fn dispatch_workgroups_for(
    batch_size: usize,
    candidates_per_invocation: u32,
    workgroup_size: u32,
) -> u32 {
    let invocations = (batch_size as u32).div_ceil(candidates_per_invocation.max(1));
    invocations.div_ceil(workgroup_size.max(1))
}

#[cfg(test)]
mod tests {
    use super::GpuDutyCycle;
    use std::time::Duration;

    #[test]
    fn gpu_duty_cycle_adds_expected_idle_time() {
        assert_eq!(
            GpuDutyCycle::idle_for(Duration::from_millis(90), 90),
            Duration::from_millis(10)
        );
    }

    #[test]
    fn gpu_duty_cycle_does_not_throttle_full_usage() {
        assert_eq!(
            GpuDutyCycle::idle_for(Duration::from_millis(50), 100),
            Duration::ZERO
        );
    }

    #[test]
    fn gpu_duty_cycle_rejects_invalid_limits() {
        assert!(GpuDutyCycle::new(0).is_err());
        assert!(GpuDutyCycle::new(101).is_err());
    }

    #[test]
    fn throttled_gpu_search_uses_short_single_dispatch_windows() {
        let tuning = super::GpuTuning::default();

        assert_eq!(
            GpuDutyCycle::responsive_tuning(tuning, 100),
            tuning,
            "unlimited GPU searches should retain throughput tuning"
        );

        let throttled = GpuDutyCycle::responsive_tuning(tuning, 90);
        assert_eq!(throttled.batch_size, 4_096);
        assert_eq!(throttled.ring_depth, 1);
        assert_eq!(throttled.workgroup_size, tuning.workgroup_size);
        assert_eq!(
            throttled.candidates_per_invocation,
            tuning.candidates_per_invocation
        );
    }

    #[test]
    fn gpu_tuning_accounts_for_submitted_attempts_without_wrapping() {
        let tuning = super::GpuTuning {
            batch_size: 262_144,
            workgroup_size: 256,
            candidates_per_invocation: 16,
            ring_depth: 3,
        };

        assert_eq!(tuning.attempts_for_batches(3), 786_432);

        let saturated = super::GpuTuning {
            batch_size: usize::MAX,
            ..tuning
        };
        assert_eq!(saturated.attempts_for_batches(usize::MAX), u64::MAX);
    }
}
