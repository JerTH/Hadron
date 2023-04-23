use std::{
    sync::{
        Mutex, 
        mpsc::{
            Sender, 
            Receiver, 
            self
        }, 
        Arc
    }, 
    thread::{
        self, 
        JoinHandle
    }, 
    panic::{
        PanicInfo, 
    }, 
    path::Path, 
    fs::File, 
    io::Write, 
    time::{
        Duration, Instant, self, SystemTime
    }, fmt::Debug
};

use once_cell::sync::Lazy;
use serde::{Serialize, Deserialize};

use self::structured::StructuredLogMessage;

static GLOBAL_LOG: Lazy<Mutex<Option<LogHandle>>> = Lazy::new(|| Mutex::new(None));

pub struct Logger {
    tx: Sender<StructuredLogMessage>,
    topic: String,
}

impl Default for Logger {
    fn default() -> Self {
        get()
    }
}

impl Logger {
    pub fn info<T>(&self, info: T) where T: Into<String> {
        let mut message = StructuredLogMessage {
            time: Logger::time_stamp_now(),
            level: structured::LogKind::Information,
            topic: self.topic.clone(),
            message: info.into(),
        };

        self.tx.send(message).expect("unable to send log message");
    }

    pub fn warn<T>(&self, info: T) where T: Into<String> {
        let mut message = StructuredLogMessage {
            time: Logger::time_stamp_now(),
            level: structured::LogKind::Warning,
            topic: self.topic.clone(),
            message: info.into(),
        };

        self.tx.send(message).expect("unable to send log message");
    }

    pub fn error<T>(&self, info: T) where T: Into<String> {
        let mut message = StructuredLogMessage {
            time: Logger::time_stamp_now(),
            level: structured::LogKind::Error,
            topic: self.topic.clone(),
            message: info.into(),
        };

        self.tx.send(message).expect("unable to send log message");
    }

    pub fn state<T, S>(&self, message: T, item: &S)
    where 
        T: Into<String>,
        S: Serialize + Debug,
    {
        let item_state = serde_json::to_string(item).expect(format!("unable to serialize {:?}", item).as_str());
        
        let mut message = StructuredLogMessage {
            time: Logger::time_stamp_now(),
            level: structured::LogKind::State(item_state),
            topic: self.topic.clone(),
            message: message.into(),
        };

        self.tx.send(message).expect("unable to send log message");
    }

    fn time_stamp_now() -> Duration {
        SystemTime::now().duration_since(time::UNIX_EPOCH).unwrap()
    }
}

#[derive(Serialize, Deserialize, PartialEq, Debug)]
struct StructuredItemState {
    item: String,
}

enum LogMessage {
    Info(String),
    Warning(String),
    Error(String),
    //Panic(PanicInfoMessage),
    State(StructuredItemState),
}

struct LogHandle {
    tx: Sender<StructuredLogMessage>,
    join_handle: Option<JoinHandle<()>>,
}

pub fn get() -> Logger {
    let tx = {
        match GLOBAL_LOG.lock() {
            Ok(mut guard) => {
                if let Some(ref sink) = *guard {
                    sink.tx.clone()
                } else {
                    let (tx, rx) = mpsc::channel();
                    let join_handle = thread::spawn(|| structured::log_receiver(rx));
                    let log_handle = LogHandle { tx: tx.clone(), join_handle: Some(join_handle) };
                    *guard = Some(log_handle);
                    set_panic_hook(tx.clone());
                    tx.clone()
                }
            },
            Err(err) => {
                panic!("unable to lock log handle: {}", err);
            },
        }
    };
    Logger {
        tx: tx,
        topic: String::from("general"),
    }
}

fn set_panic_hook(tx: Sender<structured::StructuredLogMessage>) {
    let default_panic_hook = std::panic::take_hook();

    let tx_panic = Arc::new(Mutex::new(tx));

    std::panic::set_hook(Box::new(move |info| {
        signal_panic(tx_panic.clone(), info);
        default_panic_hook(info);
    }));
}

fn signal_panic(tx: Arc<Mutex<Sender<StructuredLogMessage>>>, panic_info: &PanicInfo) {
    dbg!(panic_info);

    let structured_info = structured::StructuredPanicInfo::from_panic_info(panic_info);
    let message = structured_info.message();
    
    let panic_message = StructuredLogMessage {
        time: SystemTime::now().duration_since(time::UNIX_EPOCH).unwrap(),
        level: structured::LogKind::Panic(Arc::new(structured_info)),
        topic: String::from("panic"),
        message: message,
    };

    match tx.lock() {
        Ok(guard) => {
            match guard.send(panic_message) {
                Ok(_) => {
                    join_global_log_handle()
                },
                Err(err) => {
                    panic!("{:?}", err);
                },
            }
        },
        Err(_) => todo!(),
    }
}

fn join_global_log_handle() {
    match GLOBAL_LOG.lock() {
        Ok(mut guard) => {
            let log_handle = guard.take().expect("no log handle");
            let join_handle = log_handle.join_handle.expect("no log thread handle");
            join_handle.join().expect("unable to join log thread handle");
        },
        Err(err) => {
            panic!("unable to lock global log handle: {}", err);
        },
    }
}


mod structured {
    use std::{time::Duration, thread::ThreadId, sync::{mpsc::Receiver, Arc}, fs::{File, OpenOptions, self}, path::Path, io::{Write, Read}, panic::PanicInfo, fmt::Debug, any::Any, backtrace::Backtrace};
    
    use serde::{Serialize, Deserialize};

    use crate::unique::UniqueId;

    use super::StructuredItemState;

    #[derive(Serialize, Deserialize, PartialEq, Debug)]
    pub enum LogKind {
        Error,
        Warning,
        Information,
        Panic(Arc<StructuredPanicInfo>),
        State(String)
    }

    #[derive(Serialize, Deserialize, PartialEq, Debug)]
    pub struct StructuredPanicInfo{
        line: u32,
        file: String,
        message: String,
        backtrace: String,
    }

    impl StructuredPanicInfo {
        pub fn from_panic_info(panic_info: &PanicInfo) -> Self {
            let location = panic_info.location();
            let payload = panic_info.payload();
            let info = StructuredPanicInfo {
                line: location.map_or_else(|| 0, |l| l.line()),
                file: location.map_or_else(|| String::from("no file data"), |l| String::from(l.file())),
                message: payload.downcast_ref::<&str>().map_or_else(|| String::from("no string payload data"), |p| String::from(*p)),
                backtrace: Backtrace::force_capture().to_string(),
            };
            info
        }

        pub fn message(&self) -> String {
            self.message.clone()
        }
    }

    #[derive(Serialize, Deserialize, PartialEq, Debug)]
    pub struct StructuredLogMessage {
        pub time: Duration,
        pub level: LogKind,
        pub topic: String,
        pub message: String,
    }

    #[derive(Serialize, Deserialize, PartialEq, Debug)]
    struct StructuredLogOutput {
        pub index: usize,
        pub message: StructuredLogMessage,
    }

    #[derive(Serialize, Deserialize, PartialEq, Debug)]
    struct LogData {
        id: UniqueId,
        timestamp: chrono::DateTime<chrono::Utc>,
        messages: Vec<StructuredLogOutput>
    }

    pub fn log_receiver(rx: Receiver<StructuredLogMessage>) {
        let mut buffer: Vec<StructuredLogOutput> = Vec::new();
        let mut message_count = 0usize;
        let mut panicking = false;
        let mut next_write = 0usize;
        let skip = 1usize;
        let path = Path::new("log.json");

        create_log_file(path);

        loop {
            let message = match rx.recv() {
                Ok(message) => {
                    message
                },
                Err(err) => {
                    panic!("log receiver error: {}", err)
                },
            };

            panicking = match &message.level {
                LogKind::Panic(_) => {
                    true
                },
                _ => false,
            };

            let output = StructuredLogOutput {
                index: message_count,
                message: message,
            };

            buffer.push(output);
            
            message_count += 1;

            // Write to file if we're panicking (we might not get another chance) or we've accumulated enough messages in the buffer
            if panicking || (buffer.len() > next_write)  {
                
                next_write += skip;

                let mut data = read_log_data(path);
                data.messages.append(&mut buffer);
                
                write_log_data_truncated(path, data);
                
                buffer.clear();
            }
            
            if panicking {
                break;
            }
        }
    }

    fn create_log_file(path: &Path) {
        let data = LogData {
            id: UniqueId::get(),
            timestamp: chrono::Utc::now(),
            messages: Vec::new(),
        };
        
        let old_path = path.with_extension("json.old");

        match std::fs::remove_file(&old_path) {
            Ok(_) => (),
            Err(err) => println!("{}", err),
        }
        match std::fs::rename(path, &old_path) {
            Ok(_) => (),
            Err(err) => println!("{}", err),
        }

        let mut file = File::create(path).expect("unable to create log.json");
        
        let buf = serde_json::to_string_pretty(&data).expect("unable to serialize json log header");

        file.write(buf.as_bytes()).expect("unable to write json log header");        
    }

    fn write_log_data_truncated(path: &Path, data: LogData) {
        let mut file = File::options()
            .create(true)
            .write(true)
            .truncate(true)
            .open(path)
            .expect(format!("unable to open log {:?} for writing", path.display()).as_str());
        
        match serde_json::to_string_pretty(&data) {
            Ok(yaml) => {
                match file.write(yaml.as_bytes()) {
                    Ok(_) => (),
                    Err(err) => panic!("unable to write log master file: {}", err),
                }
            },
            Err(err) => panic!("unable to serialize log master buffer: {}", err),
        }
    }

    fn read_log_data(path: &Path) -> LogData {
        let mut file = File::options().read(true).open(path).expect("unable to open log.json");
        
        let mut buf = String::new();
        match file.read_to_string(&mut buf) {
            Ok(_length) => {
                match serde_json::from_str(&buf.as_str()) {
                    Ok(deserialized) => {
                        return deserialized
                    },
                    Err(err) => {
                        panic!("unable to deserialize log master: {}", err)
                    },
                }
            },
            Err(err) => panic!("unable to read log master: {}", err),
        }
    }
}
