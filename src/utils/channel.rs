use std::{
    collections::LinkedList,
    sync::{Arc, Condvar, Mutex},
};

#[derive(Debug)]
pub struct Channel<T> {
    buffer: Arc<Mutex<LinkedList<T>>>,
    condvar: Arc<Condvar>,
}

impl<T> Clone for Channel<T> {
    fn clone(&self) -> Self {
        Self {
            buffer: self.buffer.clone(),
            condvar: self.condvar.clone(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Sender<T> {
    channel: Channel<T>,
}

#[derive(Debug)]
pub struct Receiver<T> {
    channel: Channel<T>,
}

impl<T> Sender<T> {
    pub fn send(&self, t: T) {
        let mut buffer = self.channel.buffer.lock().unwrap();
        buffer.push_back(t);
        self.channel.condvar.notify_one();
    }
}

impl<T> Receiver<T> {
    pub fn avaliable(&self) -> bool {
        let buffer = self.channel.buffer.lock().unwrap();
        !buffer.is_empty()
    }

    pub fn clear(&self) {
        let mut buffer = self.channel.buffer.lock().unwrap();
        buffer.clear();
    }

    pub fn recv(&self) -> T {
        let mut buffer = self.channel.buffer.lock().unwrap();
        if buffer.is_empty() {
            buffer = self.channel.condvar.wait(buffer).unwrap();
        }
        buffer.pop_front().unwrap()
    }
}

pub fn channel<T>() -> (Sender<T>, Receiver<T>) {
    let channel: Channel<T> = Channel {
        buffer: Arc::new(Mutex::new(LinkedList::new())),
        condvar: Arc::new(Condvar::new()),
    };
    (
        Sender {
            channel: channel.clone(),
        },
        Receiver { channel },
    )
}
