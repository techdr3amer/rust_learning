use std::error::Error;
use std::fmt;
use std::{
    sync::{mpsc, Arc, Mutex},
    thread,
};

///"My own error" for pool creation error, can be extended for nicer logging etc.
///added as a exercise
#[derive(Debug)]
pub struct PoolCreationError;
impl Error for PoolCreationError{}
impl fmt::Display for PoolCreationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "PoolCreationError! Pool size cant be zero")
    }
}

struct Worker {
    id: usize,
    handle: Option<thread::JoinHandle<()>>,
}

impl Worker {
    /// Create a new Worker and spawns a infinite thread that gonna wait for jobs
    ///
    /// id - unique identifier of worker
    /// receiver - Atomic reference(smart pointer) to mpsc receiver that gets Job
    ///
    /// Will gracefully shutdown if receiver will get an error
    ///
    fn new(id: usize, receiver: Arc<Mutex<mpsc::Receiver<Job>>>) -> Worker {
        let handle = thread::spawn(move || loop {
            let message = receiver.lock().unwrap().recv();

            match message {
                Ok(job) => {
                    println!("Worker {id} got a job; executing.");
                    job();
                }
                Err(_) => {
                    println!("Worker {id} disconnected; shutting down.");
                    break;
                }
            }
        });
        Worker { id, handle: Some(handle),}
    }
}

// This shit is wild, kinda sick that today 30.5.2023 I know how to read this
// For me after that day: Smart pointer to Fn that implements Send trait and has static lifetime
type Job = Box<dyn FnOnce() + Send + 'static>;

pub struct ThreadPool {
    workers: Vec<Worker>,
    sender: Option<mpsc::Sender<Job>>,
}

impl ThreadPool {
    /// Create a new ThreadPool.
    ///
    /// The size is the number of threads in the pool.
    ///
    /// # Panics
    ///
    /// The `new` function will panic if the size is zero.
    pub fn new(size: usize) -> ThreadPool {
        assert!(size > 0);
        ThreadPool::create_pool(size)
    }

    /// Create a new ThreadPool, using builder idea with Result<T, E> not panic.
    ///
    /// The size is the number of threads in the pool.
    ///
    pub fn build(size: usize) -> Result<ThreadPool, PoolCreationError>{
        if size==0 {
            Err(PoolCreationError)
        }
        else{
            Ok(ThreadPool::create_pool(size))
        }
    }

    /// Helper function that returns new ThreadPool, only called in pub interfaces
    ///
    /// The size is the number of threads in the pool.
    ///
    fn create_pool(size: usize) -> ThreadPool{
        let mut workers = Vec::with_capacity(size);
        let (sender, receiver) = mpsc::channel();
        let receiver = Arc::new(Mutex::new(receiver));
        for i in 0..size {
            workers.push(Worker::new(i, Arc::clone(&receiver)))
        }
        ThreadPool { workers , sender: Some(sender),}
    }

    /// Will pass function/closure F into the sender for execution on Threads
    pub fn execute<F>(&self, f: F)
    where
        F: FnOnce() + Send + 'static,
    {
        let job = Box::new(f);
        
        self.sender.as_ref().unwrap().send(job).unwrap();
    }
}

/// Graceful "Destructor"
impl Drop for ThreadPool {
    fn drop(&mut self) {

        drop(self.sender.take());

        for worker in &mut self.workers {
            print!("Shutting down worker {}", worker.id);
            if let Some(thread) = worker.handle.take() {
                thread.join().unwrap();
            }
        }
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[should_panic]
    fn thread_pool_panics() {
        ThreadPool::new(0);
    }

    #[test]
    fn thread_pool_builder(){
        assert!(ThreadPool::build(1).is_ok());
        assert!(ThreadPool::build(0).is_err());
    }

    #[test]
    fn thread_pool_executes(){
        let pool = ThreadPool::build(1).unwrap();
        pool.execute(|| {});
    }
}