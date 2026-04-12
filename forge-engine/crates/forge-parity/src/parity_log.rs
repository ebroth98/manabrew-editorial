use std::cell::RefCell;
use std::sync::{Arc, Mutex};

thread_local! {
    static SINK: RefCell<Option<Arc<Mutex<Vec<String>>>>> = RefCell::new(None);
}

pub fn set_sink(sink: Arc<Mutex<Vec<String>>>) {
    SINK.with(|s| *s.borrow_mut() = Some(sink));
}

pub fn clear_sink() {
    SINK.with(|s| *s.borrow_mut() = None);
}

pub fn log(message: &str) {
    SINK.with(|s| {
        if let Some(ref sink) = *s.borrow() {
            sink.lock().unwrap().push(message.to_string());
        }
    });
}

pub fn drain() -> Vec<String> {
    SINK.with(|s| {
        if let Some(ref sink) = *s.borrow() {
            let mut guard = sink.lock().unwrap();
            guard.drain(..).collect()
        } else {
            Vec::new()
        }
    })
}
