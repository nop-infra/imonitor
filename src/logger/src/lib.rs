use tracing::dispatcher::Dispatch;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::prelude::__tracing_subscriber_SubscriberExt;
use tracing_subscriber::{filter::EnvFilter, filter::LevelFilter, fmt::format, prelude::*};

#[derive(Debug)]
pub struct Logger {
    pub file_path: String,
    pub dispatch: Dispatch,
    _guard: WorkerGuard,
}

impl Logger {
    pub fn new(dir: &str, file_name: &str) -> Self {
        let file_appender = tracing_appender::rolling::never(dir, file_name);
        let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);
        let filter = EnvFilter::from_default_env().add_directive(LevelFilter::INFO.into());

        let fmt_layer = tracing_subscriber::fmt::Layer::default()
            .with_writer(non_blocking)
            .with_ansi(false)
            .with_target(false)
            .with_level(true)
            //.with_thread_ids(true)
            .event_format(format().compact())
            .with_filter(filter);

        let subscriber = tracing_subscriber::Registry::default().with(fmt_layer);
        let dispatch = Dispatch::new(subscriber);

        Self {
            file_path: file_name.to_string(),
            dispatch,
            _guard: guard,
        }
    }
}

pub trait HasLogger {
    fn logger(&self) -> Option<&Logger>;
}

#[macro_export]
macro_rules! log_this {
    ($self:expr, $level:ident, $($arg:tt)*) => {{
        match $self.logger() {
            Some(logger) => {
                let dispatch = logger.dispatch.clone();
                tracing::dispatcher::with_default(&dispatch, || {
                    tracing::$level!($($arg)*);
                });
            },
            None => {
                eprintln!("Logger not set");
            }
        }
    }};
}

#[macro_export]
macro_rules! info {
    ($self:expr, $($arg:tt)+) => {{
        match $self.logger() {
            Some(logger) => {
                let dispatch = logger.dispatch.clone();
                tracing::dispatcher::with_default(&dispatch, || {
                    tracing::info!($($arg)*);
                });
            },
            None => {
                eprintln!("Logger not set");
            }
        }
    }};
}

#[macro_export]
macro_rules! debug {
    ($self:expr, $($arg:tt)+) => {{
        match $self.logger() {
            Some(logger) => {
                let dispatch = logger.dispatch.clone();
                tracing::dispatcher::with_default(&dispatch, || {
                    tracing::debug!($($arg)*);
                });
            },
            None => {
                eprintln!("Logger not set");
            }
        }
    }};
}

#[macro_export]
macro_rules! error {
    ($self:expr, $($arg:tt)+) => {{
        match $self.logger() {
            Some(logger) => {
                let dispatch = logger.dispatch.clone();
                tracing::dispatcher::with_default(&dispatch, || {
                    tracing::error!($($arg)*);
                });
            },
            None => {
                eprintln!("Logger not set");
            }
        }
    }};
}
