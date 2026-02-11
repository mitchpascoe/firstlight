use std::collections::{HashMap, HashSet};
use std::sync::mpsc;
use std::thread;

use ratatui_image::picker::Picker;
use ratatui_image::protocol::StatefulProtocol;

use crate::esa;

pub struct PreviewManager {
    picker: Picker,
    cache: HashMap<String, StatefulProtocol>,
    pending: HashSet<String>,
    tx: mpsc::Sender<(String, image::DynamicImage)>,
    rx: mpsc::Receiver<(String, image::DynamicImage)>,
}

impl PreviewManager {
    pub fn new() -> Self {
        let picker = Picker::from_query_stdio().unwrap_or_else(|_| Picker::halfblocks());
        let (tx, rx) = mpsc::channel();
        Self {
            picker,
            cache: HashMap::new(),
            pending: HashSet::new(),
            tx,
            rx,
        }
    }

    pub fn ensure_loaded(&mut self, id: &str, url: &str) {
        while let Ok((done_id, img)) = self.rx.try_recv() {
            let protocol = self.picker.new_resize_protocol(img);
            self.pending.remove(&done_id);
            self.cache.insert(done_id, protocol);
        }

        if self.cache.contains_key(id) || self.pending.contains(id) {
            return;
        }

        self.pending.insert(id.to_string());
        let tx = self.tx.clone();
        let id = id.to_string();
        let url = url.to_string();

        thread::spawn(move || {
            let Ok(path) = esa::download(&url, "prev", &id) else {
                return;
            };
            let Ok(reader) = image::ImageReader::open(&path) else {
                return;
            };
            let Ok(img) = reader.decode() else {
                return;
            };
            let _ = tx.send((id, img));
        });
    }

    pub fn get_mut(&mut self, id: &str) -> Option<&mut StatefulProtocol> {
        self.cache.get_mut(id)
    }
}
