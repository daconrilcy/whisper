use crate::{native_engine::GpuEngine, native_ffi};

#[cfg(test)]
mod tests;

pub(crate) struct CachedGpuEngine {
    model_path: String,
    model_sha256: [u8; 32],
    candidate: Option<whisper_core::gpu_inventory::GpuRuntimeIdentity>,
    engine: GpuEngine,
}

fn same_gpu_target(
    previous: Option<&whisper_core::gpu_inventory::GpuRuntimeIdentity>,
    current: Option<&whisper_core::gpu_inventory::GpuRuntimeIdentity>,
) -> bool {
    match (previous, current) {
        (None, None) => true,
        (Some(previous), Some(current)) => {
            previous
                .pnp_instance_id
                .eq_ignore_ascii_case(&current.pnp_instance_id)
                && previous.dxgi_rank == current.dxgi_rank
                && previous.luid == current.luid
                && previous.pci_id.eq_ignore_ascii_case(&current.pci_id)
                && previous.cuda_ordinal == current.cuda_ordinal
                && previous.ggml_ordinal == current.ggml_ordinal
        }
        _ => false,
    }
}

fn attest_cached<T>(
    cache: &mut Option<T>,
    attest: impl FnOnce(&T) -> Result<(), String>,
) -> Result<(), String> {
    if let Err(error) = attest(cache.as_ref().expect("cached GPU engine")) {
        *cache = None;
        return Err(error);
    }
    Ok(())
}

pub(crate) fn cached_gpu_engine<'a>(
    cache: &'a mut Option<CachedGpuEngine>,
    model_path: &str,
    model_sha256: [u8; 32],
    candidate: Option<&whisper_core::gpu_inventory::GpuRuntimeIdentity>,
) -> Result<&'a GpuEngine, String> {
    let selected_candidate = candidate.cloned();
    let (selected_ordinal, expected_backend) = match selected_candidate.as_ref() {
        Some(identity) => {
            native_ffi::whisper_gpu_device_index(identity.ggml_ordinal, &identity.pci_id)?
        }
        None => (0, "CUDA0".to_owned()),
    };
    let matches_cache = cache.as_ref().is_some_and(|cached| {
        cached.model_path == model_path
            && cached.model_sha256 == model_sha256
            && same_gpu_target(cached.candidate.as_ref(), selected_candidate.as_ref())
            && cached
                .engine
                .matches_device(selected_ordinal, &expected_backend)
    });
    if matches_cache {
        attest_cached(cache, |cached| cached.engine.attest())?;
    } else {
        *cache = None;
        eprintln!("startup_perf model_cache=load backend=gpu");
        let engine = GpuEngine::load(model_path, selected_ordinal, expected_backend)?;
        engine.attest()?;
        *cache = Some(CachedGpuEngine {
            model_path: model_path.to_owned(),
            model_sha256,
            candidate: selected_candidate,
            engine,
        });
    }
    if matches_cache {
        eprintln!("startup_perf model_cache=hit backend=gpu");
    }
    Ok(&cache.as_ref().expect("cached engine").engine)
}
