use std::os::fd::AsFd;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, RecvTimeoutError};
use std::thread::JoinHandle;
use std::time::Duration;

use nix::poll::{PollFd, PollFlags, poll};
use trinity_app::error::AppError;
use trinity_app::event::{EV_MSC, EV_SYN, InputEvent};
use trinity_app::ports::{Caps, InputEventSource};
use trinity_core::node::NodeId;

/// Directory of stable input node identifiers.
pub const BY_ID_DIR: &str = "/dev/input/by-id";

/// Internal poll interval (ms); also bounds shutdown latency.
const POLL_TIMEOUT_MS: u16 = 100;

/// Maximum wait for a first event in `read_events` (ms).
const READ_TIMEOUT_MS: u64 = 50;

/// Resolves a `NodeId` (by-id name) to a device path.
pub fn resolve_node(node: &NodeId) -> PathBuf {
    Path::new(BY_ID_DIR).join(node.as_str())
}

/// Capabilities advertised by a node (keys, relative, absolute).
pub fn node_caps(node: &NodeId) -> Result<Caps, AppError> {
    let device = open_node(node)?;
    let mut caps = Caps::new();
    if let Some(keys) = device.supported_keys() {
        for key in keys.iter() {
            caps.add_key_code(key.0);
        }
    }
    if let Some(rels) = device.supported_relative_axes() {
        for rel in rels.iter() {
            caps.add_rel(rel.0);
        }
    }
    if let Some(abss) = device.supported_absolute_axes() {
        for abs in abss.iter() {
            caps.add_abs(abs.0);
        }
    }
    Ok(caps)
}

fn open_node(node: &NodeId) -> Result<evdev::Device, AppError> {
    evdev::Device::open(resolve_node(node))
        .map_err(|err| AppError::Port(format!("opening '{node}': {err}")))
}

/// evdev source: exclusive grab of the nodes, read via a poll thread.
///
/// A single `poll(2)` thread watches every grabbed fd and pushes converted
/// events into a channel; `release` wakes the thread through the shutdown
/// flag and releases the grabs by dropping the devices.
pub struct EvdevSource {
    receiver: Option<Receiver<InputEvent>>,
    shutdown: Option<Arc<AtomicBool>>,
    join: Option<JoinHandle<()>>,
    nodes: Vec<NodeId>,
}

impl EvdevSource {
    pub fn new() -> Self {
        Self {
            receiver: None,
            shutdown: None,
            join: None,
            nodes: Vec::new(),
        }
    }
}

impl Default for EvdevSource {
    fn default() -> Self {
        Self::new()
    }
}

impl InputEventSource for EvdevSource {
    fn grab(&mut self, nodes: &[NodeId]) -> Result<(), AppError> {
        if self.join.is_some() {
            return Err(AppError::InvalidState("source already grabbed".into()));
        }
        let mut devices = Vec::with_capacity(nodes.len());
        for node in nodes {
            let mut device = open_node(node)?;
            device
                .grab()
                .map_err(|err| AppError::Port(format!("grabbing '{node}': {err}")))?;
            devices.push((node.clone(), device));
        }

        let (sender, receiver) = std::sync::mpsc::channel();
        let shutdown = Arc::new(AtomicBool::new(false));
        let flag = Arc::clone(&shutdown);
        let join = std::thread::Builder::new()
            .name("evdev-reader".into())
            .spawn(move || reader_loop(devices, sender, flag))
            .map_err(|err| AppError::Port(format!("starting reader: {err}")))?;

        self.receiver = Some(receiver);
        self.shutdown = Some(shutdown);
        self.join = Some(join);
        self.nodes = nodes.to_vec();
        Ok(())
    }

    fn release(&mut self) -> Result<(), AppError> {
        if let (Some(flag), Some(join), Some(receiver)) =
            (self.shutdown.take(), self.join.take(), self.receiver.take())
        {
            flag.store(true, Ordering::Relaxed);
            drop(receiver);
            let _ = join.join();
        }
        self.nodes.clear();
        Ok(())
    }

    fn read_events(&mut self) -> Result<Vec<InputEvent>, AppError> {
        let Some(receiver) = self.receiver.as_ref() else {
            return Err(AppError::InvalidState("source not grabbed".into()));
        };
        match receiver.recv_timeout(Duration::from_millis(READ_TIMEOUT_MS)) {
            Ok(first) => {
                let mut events = vec![first];
                while let Ok(event) = receiver.try_recv() {
                    events.push(event);
                }
                Ok(events)
            }
            Err(RecvTimeoutError::Timeout) => Ok(Vec::new()),
            Err(RecvTimeoutError::Disconnected) => {
                Err(AppError::Port("evdev reader stopped".into()))
            }
        }
    }

    fn grabbed(&self) -> &[NodeId] {
        &self.nodes
    }
}

impl Drop for EvdevSource {
    fn drop(&mut self) {
        let _ = self.release();
    }
}

fn reader_loop(
    mut devices: Vec<(NodeId, evdev::Device)>,
    sender: std::sync::mpsc::Sender<InputEvent>,
    shutdown: Arc<AtomicBool>,
) {
    while !shutdown.load(Ordering::Relaxed) {
        let ready = {
            let mut fds: Vec<PollFd> = devices
                .iter()
                .map(|(_, device)| PollFd::new(device.as_fd(), PollFlags::POLLIN))
                .collect();
            match poll(&mut fds, POLL_TIMEOUT_MS) {
                Ok(_) => (0..devices.len())
                    .filter(|&index| {
                        fds[index]
                            .revents()
                            .is_some_and(|revents| revents.contains(PollFlags::POLLIN))
                    })
                    .collect::<Vec<usize>>(),
                Err(_) => break,
            }
        };
        for index in ready {
            let (node, device) = &mut devices[index];
            let Ok(events) = device.fetch_events() else {
                continue;
            };
            for event in events {
                if let Some(converted) = convert(node, &event) {
                    if sender.send(converted).is_err() {
                        return;
                    }
                }
            }
        }
    }
    for (_, mut device) in devices {
        let _ = device.ungrab();
    }
}

/// evdev to domain conversion; SYN and MSC are implicit on injection.
fn convert(node: &NodeId, event: &evdev::InputEvent) -> Option<InputEvent> {
    let event_type = event.event_type().0;
    if event_type == EV_SYN || event_type == EV_MSC {
        return None;
    }
    Some(InputEvent {
        node: node.clone(),
        event_type,
        code: event.code(),
        value: event.value(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_node_uses_by_id_dir() {
        let node = NodeId::new("usb-Razer-event-kbd").unwrap();
        assert_eq!(
            resolve_node(&node),
            Path::new("/dev/input/by-id/usb-Razer-event-kbd")
        );
    }

    #[test]
    #[ignore = "requires node access (input group)"]
    fn trinity_nodes_expose_caps() {
        let node = NodeId::new("usb-Razer_Razer_Naga_Trinity_00000000001A-event-mouse").unwrap();
        let caps = node_caps(&node).unwrap();
        assert!(caps.keys().contains(&272));
        assert!(caps.rels().contains(&0));
    }

    #[test]
    #[ignore = "requires node access (input group)"]
    fn grab_release_roundtrip() {
        let node = NodeId::new("usb-Razer_Razer_Naga_Trinity_00000000001A-event-mouse").unwrap();
        let mut source = EvdevSource::new();
        source.grab(std::slice::from_ref(&node)).unwrap();
        assert_eq!(source.grabbed(), std::slice::from_ref(&node));
        let _ = source.read_events();
        source.release().unwrap();
        assert!(source.grabbed().is_empty());
    }

    #[test]
    fn read_events_without_grab_is_rejected() {
        let mut source = EvdevSource::new();
        assert!(matches!(
            source.read_events().unwrap_err(),
            AppError::InvalidState(_)
        ));
    }

    #[test]
    fn grab_missing_node_is_a_port_error() {
        let mut source = EvdevSource::new();
        let node = NodeId::new("usb-inexistant-event-kbd").unwrap();
        assert!(matches!(
            source.grab(&[node]).unwrap_err(),
            AppError::Port(_)
        ));
    }
}
