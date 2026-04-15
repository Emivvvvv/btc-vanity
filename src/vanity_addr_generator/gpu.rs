use crate::error::VanityError;
use crate::keys_and_address::BitcoinKeyPair;
#[cfg(feature = "ethereum")]
use crate::keys_and_address::EthereumKeyPair;
use crate::keys_and_address::KeyPairGenerator;
#[cfg(feature = "solana")]
use crate::keys_and_address::SolanaKeyPair;
use crate::vanity_addr_generator::chain::VanityChain;
use crate::vanity_addr_generator::vanity_addr::{GpuSearchTarget, VanityMode};

use bitcoin::hashes::{ripemd160, sha256, Hash};
use bitcoin::secp256k1::{rand, PublicKey as SecpPublicKey, Secp256k1, SecretKey};
use bitcoin::{Address, Network, PublicKey};
use multiprecision::bigint;
use multiprecision::utils::calc_num_limbs;
use num_bigint::BigUint;
use pollster::block_on;
use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::sync::Mutex;
use crate::wgpu_sig_ops::gpu::{
    create_bind_group, create_command_encoder, create_compute_pipeline, create_empty_sb,
    create_sb_with_data, create_ub_with_data, execute_pipeline,
};
use crate::wgpu_sig_ops::precompute::{ed25519_bases, secp256k1_bases};
use crate::wgpu_sig_ops::shader::{render_ed25519_curve_tests, render_secp256k1_curve_tests};

pub const GPU_BATCH_SIZE: usize = 262_144;
pub const GPU_MAX_BATCH_SIZE: usize = 2_097_152;
pub const GPU_RING_DEPTH: usize = 4;
const CPU_FALLBACK_VERIFY_INTERVAL_BATCHES: usize = 64;
const CPU_FALLBACK_VERIFY_MAX_BATCH_SIZE: usize = 65_536;

const LOG_LIMB_SIZE: u32 = 13;
const WORKGROUP_SIZE: u32 = 256;
const PATTERN_CAPACITY: usize = 40;
const RESULT_WORDS: usize = 57;

const RESULT_WINNER_INDEX: usize = 0;
const RESULT_ATTEMPTS_LO_INDEX: usize = 1;
const RESULT_ATTEMPTS_HI_INDEX: usize = 2;
const RESULT_BATCHES_INDEX: usize = 3;
const RESULT_ADDRESS_LEN_INDEX: usize = 4;
const RESULT_SCALAR_INDEX: usize = 5;
const RESULT_ADDRESS_INDEX: usize = RESULT_SCALAR_INDEX + 8;
const SECP256K1_ORDER_HEX: &[u8] =
    b"FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFEBAAEDCE6AF48A03BBFD25E8CD0364141";

#[derive(Clone, Debug)]
pub struct GpuMatch {
    pub private_key_bytes: [u8; 32],
    pub address: String,
    pub attempts: u64,
    pub batches: u64,
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
            ring_depth: GPU_RING_DEPTH,
        }
    }
}

struct DispatchSlot {
    counter_buf: wgpu::Buffer,
    params_buf: wgpu::Buffer,
    result_buf: wgpu::Buffer,
    result_readback_buf: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
    readback_state: Mutex<SlotReadbackState>,
}

enum SlotReadbackState {
    Idle,
    Pending(Receiver<Result<(), wgpu::BufferAsyncError>>),
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
    execution_lock: Mutex<()>,
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
        if !required_gpu_shader_assets_present() {
            return Err(VanityError::GpuBackendUnavailable);
        }
        let (device, queue, adapter_info) = block_on(async {
            let instance = wgpu::Instance::default();
            let adapter = request_supported_adapter(&instance, MIN_STORAGE_BUFFERS_PER_SHADER_STAGE)
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

        let ethereum_pipeline = Self::create_exact_pipeline(
            &device,
            &secp_table_buf,
            num_limbs,
            "secp256k1_eth_vanity_search.wgsl",
            "secp256k1_eth_vanity_search",
            false,
        );
        let bitcoin_pipeline = Self::create_exact_pipeline(
            &device,
            &secp_table_buf,
            num_limbs,
            "secp256k1_btc_vanity_search.wgsl",
            "secp256k1_btc_vanity_search",
            false,
        );
        let solana_pipeline = Self::create_exact_pipeline(
            &device,
            &ed25519_table_buf,
            num_limbs,
            "ed25519_sol_vanity_search.wgsl",
            "ed25519_sol_vanity_search",
            true,
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
    ) -> ExactSearchPipeline {
        let seed_buf = create_empty_sb(device, (num_limbs * std::mem::size_of::<u32>()) as u64);
        let pattern_buf =
            create_empty_sb(device, (PATTERN_CAPACITY * std::mem::size_of::<u32>()) as u64);
        let source = if is_ed25519 {
            render_ed25519_curve_tests(shader_name, LOG_LIMB_SIZE)
        } else {
            render_secp256k1_curve_tests(shader_name, LOG_LIMB_SIZE)
        };
        let compute_pipeline = create_compute_pipeline(device, &source, entry_point);
        let mut slots = Vec::with_capacity(GPU_RING_DEPTH);

        for slot_index in 0..GPU_RING_DEPTH {
            let counter_buf =
                create_empty_sb(device, (calc_num_limbs(LOG_LIMB_SIZE, 256) * 4) as u64);
            let params_buf = create_ub_with_data(device, &[0u32; 8]);
            let result_buf =
                create_empty_sb(device, (RESULT_WORDS * std::mem::size_of::<u32>()) as u64);
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
                counter_buf,
                params_buf,
                result_buf,
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
            execution_lock: Mutex::new(()),
        }
    }

    pub fn adapter_name(&self) -> String {
        format!("{:?}: {}", self.adapter_info.backend, self.adapter_info.name)
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
        self.search_exact_with_tuning(target, seed, pattern, case_sensitive, vanity_mode, tuning)
    }

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
                pattern.len(),
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
                                pattern.len(),
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
                    if self.should_verify_cpu_batch(tuning, batch_index) {
                        if let Some(found) = self.cpu_scan_exact_batch(
                            target,
                            seed,
                            pattern,
                            case_sensitive,
                            vanity_mode,
                            tuning,
                            batch_index,
                        )? {
                            return Ok(Some(found));
                        }
                    }
                    if next_batch_index < batches {
                        let new_batch_index = next_batch_index;
                        self.submit_exact_slot(
                            pipeline,
                            slot_index,
                            pattern.len(),
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

    pub(crate) fn search_exact_with_tuning(
        &self,
        target: GpuSearchTarget,
        seed: [u8; 32],
        pattern: &[u8],
        case_sensitive: bool,
        vanity_mode: VanityMode,
        tuning: GpuTuning,
    ) -> Result<GpuMatch, VanityError> {
        self.validate_exact_request(pattern, vanity_mode, tuning)?;
        let pipeline = self.pipeline_for_target(target)?;
        let _execution_guard = match pipeline.execution_lock.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };
        self.write_seed(&pipeline.seed_buf, &seed);
        self.write_pattern(&pipeline.pattern_buf, pattern, case_sensitive);

        let ring_depth = tuning.ring_depth.min(pipeline.slots.len()).max(1);
        let mut inflight: VecDeque<(usize, usize)> = VecDeque::with_capacity(ring_depth);
        let mut next_batch_index = 0usize;
        let mut pending_streak = 0usize;

        while inflight.len() < ring_depth {
            let batch_index = next_batch_index;
            let slot_index = batch_index % ring_depth;
            self.submit_exact_slot(
                pipeline,
                slot_index,
                pattern.len(),
                case_sensitive,
                vanity_mode,
                tuning,
                batch_index as u32,
            );
            inflight.push_back((slot_index, batch_index));
            next_batch_index += 1;
        }

        loop {
            let Some((slot_index, batch_index)) = inflight.pop_front() else {
                return Err(VanityError::GpuInvalidResult(
                    "GPU dispatch ring unexpectedly empty",
                ));
            };

            match self.inspect_slot(pipeline, slot_index)? {
                SlotInspection::Pending => {
                    pending_streak += 1;
                    inflight.push_back((slot_index, batch_index));
                }
                SlotInspection::Ready(Some(found)) => {
                    if !self.is_valid_match_for_target(target, &found) {
                        let new_batch_index = next_batch_index;
                        self.submit_exact_slot(
                            pipeline,
                            slot_index,
                            pattern.len(),
                            case_sensitive,
                            vanity_mode,
                            tuning,
                            new_batch_index as u32,
                        );
                        inflight.push_back((slot_index, new_batch_index));
                        next_batch_index += 1;
                        continue;
                    }
                    self.cleanup_inflight_readbacks(pipeline, &inflight);
                    return Ok(found);
                }
                SlotInspection::Ready(None) => {
                    pending_streak = 0;
                    if self.should_verify_cpu_batch(tuning, batch_index) {
                        if let Some(found) = self.cpu_scan_exact_batch(
                            target,
                            seed,
                            pattern,
                            case_sensitive,
                            vanity_mode,
                            tuning,
                            batch_index,
                        )? {
                            return Ok(found);
                        }
                    }
                    let new_batch_index = next_batch_index;
                    self.submit_exact_slot(
                        pipeline,
                        slot_index,
                        pattern.len(),
                        case_sensitive,
                        vanity_mode,
                        tuning,
                        new_batch_index as u32,
                    );
                    inflight.push_back((slot_index, new_batch_index));
                    next_batch_index += 1;
                }
            }

            if pending_streak >= inflight.len().max(1) {
                self.wait_for_gpu_progress();
                pending_streak = 0;
            }
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
            return Err(VanityError::GpuInvalidResult("pattern is too long for GPU search"));
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
        let seed_limbs = bigint::from_biguint_le(&scalar, self.num_limbs, LOG_LIMB_SIZE);
        self.queue
            .write_buffer(seed_buf, 0, bytemuck::cast_slice(&seed_limbs));
    }

    fn write_counter(&self, counter_buf: &wgpu::Buffer, attempt_offset: u64) {
        let counter_limbs = bigint::from_biguint_le(
            &BigUint::from(attempt_offset),
            self.num_limbs,
            LOG_LIMB_SIZE,
        );
        self.queue
            .write_buffer(counter_buf, 0, bytemuck::cast_slice(&counter_limbs));
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
        let mut zeros = [0u32; RESULT_WORDS];
        zeros[RESULT_WINNER_INDEX] = u32::MAX;
        self.queue
            .write_buffer(result_buf, 0, bytemuck::cast_slice(&zeros));
    }

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
        self.write_counter(&slot.counter_buf, attempt_offset);
        self.reset_result_buffer(&slot.result_buf);
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

        let mut state = match slot.readback_state.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };
        *state = SlotReadbackState::Pending(receiver);
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

        let poll_result = match &mut *state {
            SlotReadbackState::Idle => return Ok(SlotInspection::Pending),
            SlotReadbackState::Pending(receiver) => receiver.try_recv(),
        };

        match poll_result {
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
                    "GPU readback channel disconnected",
                ))
            }
        }
    }

    fn cpu_scan_exact_batch(
        &self,
        target: GpuSearchTarget,
        seed: [u8; 32],
        pattern: &[u8],
        case_sensitive: bool,
        vanity_mode: VanityMode,
        tuning: GpuTuning,
        batch_index: usize,
    ) -> Result<Option<GpuMatch>, VanityError> {
        let attempt_offset = batch_index as u64 * tuning.batch_size as u64;
        for lane in 0..tuning.batch_size {
            let attempts = attempt_offset + lane as u64 + 1;
            let private_key_bytes = secp256k1_scalar_from_seed(seed, attempt_offset + lane as u64);
            let address = derive_address_for_target(target, private_key_bytes)?;
            if !matches_pattern(address.as_bytes(), pattern, case_sensitive, vanity_mode) {
                continue;
            }

            return Ok(Some(GpuMatch {
                private_key_bytes,
                address,
                attempts,
                batches: batch_index as u64 + 1,
            }));
        }

        Ok(None)
    }

    fn should_verify_cpu_batch(&self, tuning: GpuTuning, batch_index: usize) -> bool {
        if tuning.batch_size <= 1_024 {
            return true;
        }

        if tuning.batch_size > CPU_FALLBACK_VERIFY_MAX_BATCH_SIZE {
            return false;
        }

        batch_index.is_multiple_of(CPU_FALLBACK_VERIFY_INTERVAL_BATCHES)
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
        matches!(*state, SlotReadbackState::Pending(_))
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

fn derive_address_for_target(
    target: GpuSearchTarget,
    private_key_bytes: [u8; 32],
) -> Result<String, VanityError> {
    match target {
        GpuSearchTarget::Bitcoin => {
            let candidate = <BitcoinKeyPair as VanityChain>::from_private_key_bytes(private_key_bytes)?;
            Ok(candidate.get_address().clone())
        }
        GpuSearchTarget::Ethereum => {
            #[cfg(feature = "ethereum")]
            {
                let candidate =
                    <EthereumKeyPair as VanityChain>::from_private_key_bytes(private_key_bytes)?;
                Ok(candidate.get_address().clone())
            }
            #[cfg(not(feature = "ethereum"))]
            {
                let _ = private_key_bytes;
                Err(VanityError::GpuInvalidResult(
                    "ethereum address recovery requested without ethereum feature",
                ))
            }
        }
        GpuSearchTarget::Solana => {
            #[cfg(feature = "solana")]
            {
                let candidate =
                    <SolanaKeyPair as VanityChain>::from_private_key_bytes(private_key_bytes)?;
                Ok(candidate.get_address().clone())
            }
            #[cfg(not(feature = "solana"))]
            {
                let _ = private_key_bytes;
                Err(VanityError::GpuInvalidResult(
                    "solana address recovery requested without solana feature",
                ))
            }
        }
    }
}

fn matches_pattern(
    address_bytes: &[u8],
    pattern_bytes: &[u8],
    case_sensitive: bool,
    vanity_mode: VanityMode,
) -> bool {
    if pattern_bytes.is_empty() {
        return true;
    }

    if case_sensitive {
        return match vanity_mode {
            VanityMode::Prefix => address_bytes.starts_with(pattern_bytes),
            VanityMode::Suffix => address_bytes.ends_with(pattern_bytes),
            VanityMode::Anywhere => address_bytes
                .windows(pattern_bytes.len())
                .any(|window| window == pattern_bytes),
            VanityMode::Regex => false,
        };
    }

    if pattern_bytes.len() > address_bytes.len() {
        return false;
    }

    match vanity_mode {
        VanityMode::Prefix => address_bytes
            .iter()
            .zip(pattern_bytes.iter())
            .all(|(address, pattern)| address.eq_ignore_ascii_case(pattern)),
        VanityMode::Suffix => {
            let start = address_bytes.len() - pattern_bytes.len();
            address_bytes[start..]
                .iter()
                .zip(pattern_bytes.iter())
                .all(|(address, pattern)| address.eq_ignore_ascii_case(pattern))
        }
        VanityMode::Anywhere => {
            let last_offset = address_bytes.len() - pattern_bytes.len();
            (0..=last_offset).any(|offset| {
                address_bytes[offset..offset + pattern_bytes.len()]
                    .iter()
                    .zip(pattern_bytes.iter())
                    .all(|(address, pattern)| address.eq_ignore_ascii_case(pattern))
            })
        }
        VanityMode::Regex => false,
    }
}

fn secp256k1_scalar_from_seed(seed: [u8; 32], offset: u64) -> [u8; 32] {
    let order = BigUint::parse_bytes(SECP256K1_ORDER_HEX, 16).expect("valid secp256k1 order");
    let seed_big = BigUint::from_bytes_be(&seed);
    let mut scalar = (seed_big + BigUint::from(offset)) % &order;
    if scalar == BigUint::default() {
        scalar = BigUint::from(1u8);
    }

    let bytes = scalar.to_bytes_be();
    let mut result = [0u8; 32];
    result[32 - bytes.len()..].copy_from_slice(&bytes);
    result
}

fn required_gpu_shader_assets_present() -> bool {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/wgpu_sig_ops/wgsl");
    let required_paths = [
        "bigint.wgsl",
        "ff.wgsl",
        "mont.wgsl",
        "secp256k1_curve.wgsl",
        "secp_constants.wgsl",
        "secp_curve_utils.wgsl",
        "constants.wgsl",
        "limbs_le_to_u32s_be.wgsl",
        "bytes_be_to_limbs_le.wgsl",
        "keccak256.wgsl",
        "sha256.wgsl",
        "ripemd160.wgsl",
        "base58.wgsl",
        "vanity_match.wgsl",
        "vanity_scalar.wgsl",
        "ed25519_curve.wgsl",
        "ed25519_utils.wgsl",
        "ed25519_constants.wgsl",
        "sha512.wgsl",
        "tests/secp256k1_eth_vanity_search.wgsl",
        "tests/secp256k1_btc_vanity_search.wgsl",
        "tests/ed25519_sol_vanity_search.wgsl",
    ];

    required_paths.iter().all(|path| root.join(path).exists())
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

    let mut private_key_bytes = [0u8; 32];
    for (index, word) in result[RESULT_SCALAR_INDEX..RESULT_SCALAR_INDEX + 8]
        .iter()
        .enumerate()
    {
        private_key_bytes[index * 4..(index + 1) * 4].copy_from_slice(&word.to_le_bytes());
    }

    let address_len = result.get(RESULT_ADDRESS_LEN_INDEX).copied()? as usize;
    if address_len > RESULT_WORDS.saturating_sub(RESULT_ADDRESS_INDEX) {
        return None;
    }
    let address_end = RESULT_ADDRESS_INDEX.checked_add(address_len)?;
    if address_end > result.len() {
        return None;
    }
    let address_bytes = result[RESULT_ADDRESS_INDEX..address_end]
        .iter()
        .map(|word| *word as u8)
        .collect::<Vec<_>>();
    let address = String::from_utf8(address_bytes).ok()?;
    let attempts = ((result[RESULT_ATTEMPTS_HI_INDEX] as u64) << 32)
        | result[RESULT_ATTEMPTS_LO_INDEX] as u64;
    let batches = result[RESULT_BATCHES_INDEX] as u64;

    Some(GpuMatch {
        private_key_bytes,
        address,
        attempts,
        batches,
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
