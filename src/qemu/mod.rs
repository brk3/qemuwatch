// TODO(SamYaple): Sort replies into command responses and events
// TODO(SamYaple): Create listening socket for this daemon
// TODO(SamYaple): Launch QEMU instance after receiving command on socket
// TODO(SamYaple): Scan directory structure and (re)attach to sockets
// TODO(SamYaple): Write logs
// TODO(SamYaple): Improve error handling when talking to unix socket
// TODO(SamYaple): Error handling for QMP responses (QMP/QEMU issues, not operating system related)
// TODO(SamYaple): Write tests

extern crate serde_json;

use std::io::prelude::*;

use std::io::BufReader;
use std::os::unix::net::UnixStream;
use std::sync::mpsc;
use std::thread;

pub use self::response::QemuInfo;
pub use self::response::Info;
pub use self::response::Version;
pub use self::response::Qemu;
pub use self::response::Execute;
mod response;

fn execute(writer: &mut UnixStream, command: String) {
    let command = serde_json::to_string(&Execute{ command: command }).unwrap();
    writer.write_all(command.as_bytes()).unwrap();
}

pub fn monitor(socket_path: &str, command_rx: mpsc::Receiver<String>, reply_tx: mpsc::Sender<String>, event_tx: mpsc::Sender<String>) {
    // TODO(SamYaple): Error checking
    let mut writer = UnixStream::connect(socket_path).unwrap();
    let read_socket = writer.try_clone().unwrap();
    let mut reader = BufReader::new(read_socket);

    // NOTE(SamYaple): Read initial string from socket containing version and capability info
    let mut version_raw = String::new();
    match reader.read_line(&mut version_raw) {
        Ok(_)  => { },
        Err(x) => { println!("unexpected error: {}", x); return; },
    }
    let info: QemuInfo = serde_json::from_str(&version_raw).unwrap();
    let version = info.qmp.version;
    println!("Version: {}, Package:{}", version.qemu, version.package);

    // NOTE(SamYaple): At this time, there are no capabilites to negotiate, this is "future-proofing" for QMP
    // NOTE(SamYaple): Exit capability negotiation
    execute(&mut writer, "qmp_capabilities".to_string());
    match reader.read_line(&mut String::new()) {
        Ok(x)  => if x == 0 { return; },
        Err(x) => {
            println!("Unknown Error: {}", x);
            return;
        },
    }

    // NOTE(SamYaple): Worker thread recieves commands from channel and writes them to the socket
    thread::spawn(move || {
        loop {
            // NOTE(SamYaple): This is a blocking operation
            let command = command_rx.recv().unwrap();
            execute(&mut writer, command);
        }
    });

    loop {
        let mut line = String::new();
        // NOTE(SamYaple): This is a blocking operation
        match reader.read_line(&mut line) {
            Ok(x)  => if x == 0 { return; },
            Err(x) => {
                println!("Unknown Error: {}", x);
                return;
            },
        }
        // TODO(SamYaple): Seperate these into events and replies
        reply_tx.send(line.trim().to_string()).unwrap();
        event_tx.send(line.trim().to_string()).unwrap();
    }
}
