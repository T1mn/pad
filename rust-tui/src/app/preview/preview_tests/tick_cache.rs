mod busy_tick {
    use super::*;
    include!("tick_cache/busy_tick.rs");
}

mod debounce_detail {
    use super::*;
    include!("tick_cache/debounce_detail.rs");
}

mod plain_cache {
    use super::*;
    include!("tick_cache/plain_cache.rs");
}

mod thread_cache {
    use super::*;
    include!("tick_cache/thread_cache.rs");
}
