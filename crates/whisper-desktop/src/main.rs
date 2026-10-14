mod root;

fn main() -> eframe::Result {
    let (application, compute_choice, startup_message) = root::compose();
    whisper_desktop::run(application, compute_choice, startup_message)
}
