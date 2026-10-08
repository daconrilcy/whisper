mod root;

fn main() -> eframe::Result {
    whisper_desktop::run(root::compose())
}
