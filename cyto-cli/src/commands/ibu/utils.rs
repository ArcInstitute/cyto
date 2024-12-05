pub fn init_thread_pool(num_threads: usize) -> Result<(), rayon::ThreadPoolBuildError> {
    rayon::ThreadPoolBuilder::new()
        .num_threads(num_threads)
        .build_global()
}
