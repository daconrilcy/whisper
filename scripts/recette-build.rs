fn main() {
    if std::env::var_os("CARGO_FEATURE_RECETTE_HOOKS").is_some()
        && std::env::var_os("PROFILE").as_deref() == Some(std::ffi::OsStr::new("release"))
    {
        panic!("recette-hooks is forbidden in Cargo release profiles");
    }
}
