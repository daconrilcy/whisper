use std::thread;

impl super::PcmWriter {
    pub(crate) fn observe_shutdown(&mut self, callback: impl FnOnce() + Send + 'static) {
        let Some(writer_worker) = self.worker.take() else {
            return;
        };
        self.worker = Some(thread::spawn(move || {
            let _ = writer_worker.join();
            callback();
        }));
    }
}
