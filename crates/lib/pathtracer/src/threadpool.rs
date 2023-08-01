use std::sync::{mpsc::*, Arc, Mutex};
use std::thread::JoinHandle;

type Job = Box<dyn FnOnce() + Send + 'static>;

enum Task {
    Work(Job),
    Terminate,
}

struct Worker {
    _id: usize,
    thread: Option<JoinHandle<()>>,
}

pub struct ThreadPool {
    workers: Vec<Worker>,
    sender: Sender<Task>,
}

impl Worker {
    fn new(id: usize, receiver: Arc<Mutex<Receiver<Task>>>) -> Worker {
        let thread = std::thread::spawn(move || 'top: loop {
            // if id == 0 {
            //     optick::register_thread(&format!("Worker {}", id));
            // }

            let job = receiver.lock().unwrap().recv().unwrap();

            match job {
                Task::Work(c) => c(),
                Task::Terminate => break 'top,
            };
        });

        Worker {
            _id: id,
            thread: Some(thread),
        }
    }
}

impl ThreadPool {
    fn thread_count() -> usize {
        match std::thread::available_parallelism() {
            Ok(val) => val.get() as usize,
            _ => 4,
        }
    }

    pub fn new(cnt: Option<usize>) -> ThreadPool {
        let thread_cnt = cnt.unwrap_or_else(Self::thread_count);

        println!("Spawning {} threads", thread_cnt);

        let mut workers = Vec::with_capacity(thread_cnt);

        let (sender, receiver) = channel();
        let receiver = Arc::new(Mutex::new(receiver));

        for id in 0..thread_cnt {
            workers.push(Worker::new(id, Arc::clone(&receiver)));
        }

        ThreadPool { workers, sender }
    }

    pub fn schedule<F: FnOnce() + Send + 'static>(&self, work: F) {
        let job = Box::new(work);

        self.sender.send(Task::Work(job)).unwrap();
    }
}

impl Drop for ThreadPool {
    fn drop(&mut self) {
        println!("Dropping thread pool...");

        for _ in &self.workers {
            self.sender.send(Task::Terminate).unwrap();
        }

        for t in &mut self.workers {
            let thr = t.thread.take().unwrap();
            thr.join().unwrap();
        }

        println!("Pool destroyed",);
    }
}
