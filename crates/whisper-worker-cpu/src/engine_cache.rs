use crate::native_engine::CpuEngine;

pub(crate) fn cached_cpu_engine<'a>(
    cache: &'a mut Option<(String, [u8; 32], CpuEngine)>,
    model_path: &str,
    model_sha256: [u8; 32],
) -> Result<&'a CpuEngine, String> {
    if cache
        .as_ref()
        .is_none_or(|(path, hash, _)| path != model_path || hash != &model_sha256)
    {
        *cache = None;
        eprintln!("startup_perf model_cache=load backend=cpu");
        *cache = Some((
            model_path.to_owned(),
            model_sha256,
            CpuEngine::load(model_path)?,
        ));
    } else {
        eprintln!("startup_perf model_cache=hit backend=cpu");
    }
    Ok(&cache.as_ref().expect("cached engine").2)
}
